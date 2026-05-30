use tune_hir::{ExprId, HirId, MemberId};
use tune_ir::Reg;

use crate::Opcode;
use crate::function::{
    BytecodeBoundCallSite, BytecodeCallSite, BytecodeCallableSite, BytecodeCapture,
    BytecodeCaptureMode, BytecodeGenericStrategy, BytecodeHostCallSite, Instruction,
};
use crate::lower::{BytecodeLowerError, FunctionLowerer};

impl FunctionLowerer<'_> {
    pub(super) fn lower_direct_call(
        &mut self,
        dst: Reg,
        function: HirId,
        args: &[Reg],
        type_args: &[tune_shape::Shape],
    ) -> Result<(), BytecodeLowerError> {
        let function = *self
            .function_indices
            .get(&function)
            .ok_or(BytecodeLowerError::UnknownFunction)?;
        self.push_call_direct(dst, function, args, type_args)
    }

    pub(super) fn lower_member_call(
        &mut self,
        dst: Reg,
        member: MemberId,
        args: &[Reg],
    ) -> Result<(), BytecodeLowerError> {
        let function = *self
            .member_indices
            .get(&member)
            .ok_or(BytecodeLowerError::UnknownFunction)?;
        self.push_call_direct(dst, function, args, &[])
    }

    pub(super) fn lower_callable_value(
        &mut self,
        dst: Reg,
        callable: ExprId,
        captures: &[tune_ir::IrCapture],
    ) -> Result<(), BytecodeLowerError> {
        let function = *self
            .callable_indices
            .get(&callable)
            .ok_or(BytecodeLowerError::UnknownFunction)?;
        let site = u32::try_from(self.callable_sites.len())
            .map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.callable_sites.push(BytecodeCallableSite {
            function,
            captures: captures
                .iter()
                .map(|capture| BytecodeCapture {
                    register: capture.reg.0,
                    mode: match capture.mode {
                        tune_ir::IrCaptureMode::Reference => BytecodeCaptureMode::Reference,
                        tune_ir::IrCaptureMode::PrivateSnapshot => {
                            BytecodeCaptureMode::PrivateSnapshot
                        }
                    },
                })
                .collect(),
        });
        self.instructions.push(Instruction {
            opcode: Opcode::CallableValue,
            a: dst.0,
            b: site,
            c: 0,
        });
        Ok(())
    }

    pub(super) fn lower_bound_call(
        &mut self,
        dst: Reg,
        callee: Reg,
        args: &[Reg],
    ) -> Result<(), BytecodeLowerError> {
        let site = u32::try_from(self.bound_call_sites.len())
            .map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.bound_call_sites.push(BytecodeBoundCallSite {
            args: args.iter().map(|arg| arg.0).collect(),
        });
        self.instructions.push(Instruction {
            opcode: Opcode::CallBound,
            a: dst.0,
            b: site,
            c: callee.0,
        });
        Ok(())
    }

    pub(super) fn lower_host_call(
        &mut self,
        dst: Reg,
        symbol: tune_ir::HostSymbolId,
        task_safe: bool,
        args: &[Reg],
    ) -> Result<(), BytecodeLowerError> {
        let site = u32::try_from(self.host_call_sites.len())
            .map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.host_call_sites.push(BytecodeHostCallSite {
            symbol,
            task_safe,
            args: args.iter().map(|arg| arg.0).collect(),
        });
        self.instructions.push(Instruction {
            opcode: Opcode::CallHost,
            a: dst.0,
            b: site,
            c: 0,
        });
        Ok(())
    }

    fn push_call_direct(
        &mut self,
        dst: Reg,
        function: u32,
        args: &[Reg],
        type_args: &[tune_shape::Shape],
    ) -> Result<(), BytecodeLowerError> {
        let call_site =
            u32::try_from(self.call_sites.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.call_sites.push(BytecodeCallSite {
            function,
            args: args.iter().map(|arg| arg.0).collect(),
            type_args: type_args.to_vec(),
            generic_strategy: generic_strategy(type_args),
        });
        self.instructions.push(Instruction {
            opcode: Opcode::CallDirect,
            a: dst.0,
            b: call_site,
            c: 0,
        });
        Ok(())
    }
}

fn generic_strategy(type_args: &[tune_shape::Shape]) -> BytecodeGenericStrategy {
    if type_args.is_empty() {
        return BytecodeGenericStrategy::None;
    }
    if type_args.iter().any(shape_contains_type_param) {
        BytecodeGenericStrategy::WitnessShared
    } else {
        BytecodeGenericStrategy::DirectSpecialization
    }
}

fn shape_contains_type_param(shape: &tune_shape::Shape) -> bool {
    match shape {
        tune_shape::Shape::Param(_) => true,
        tune_shape::Shape::Sequence(inner)
        | tune_shape::Shape::Range(inner)
        | tune_shape::Shape::Optional(inner)
        | tune_shape::Shape::Task(inner) => shape_contains_type_param(inner),
        tune_shape::Shape::Tuple(items) | tune_shape::Shape::Union(items) => {
            items.iter().any(shape_contains_type_param)
        }
        tune_shape::Shape::Callable { params, ret } => {
            params.iter().any(shape_contains_type_param) || shape_contains_type_param(ret)
        }
        tune_shape::Shape::Result { ok, err } => {
            shape_contains_type_param(ok) || shape_contains_type_param(err)
        }
        tune_shape::Shape::Apply { args, .. } => args.iter().any(shape_contains_type_param),
        tune_shape::Shape::Structural(requirements) => {
            requirements.iter().any(|requirement| match requirement {
                tune_shape::MemberRequirement::Field { shape, .. } => {
                    shape.as_ref().is_some_and(shape_contains_type_param)
                }
                tune_shape::MemberRequirement::Callable { params, ret, .. } => {
                    params.iter().any(shape_contains_type_param)
                        || ret.as_ref().is_some_and(shape_contains_type_param)
                }
            })
        }
        _ => false,
    }
}
