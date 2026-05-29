use tune_hir::{ExprId, HirId, MemberId};
use tune_ir::Reg;

use crate::Opcode;
use crate::function::{BytecodeBoundCallSite, BytecodeCallSite, BytecodeCallableSite, Instruction};
use crate::lower::{BytecodeLowerError, FunctionLowerer};

impl FunctionLowerer<'_> {
    pub(super) fn lower_direct_call(
        &mut self,
        dst: Reg,
        function: HirId,
        args: &[Reg],
    ) -> Result<(), BytecodeLowerError> {
        let function = *self
            .function_indices
            .get(&function)
            .ok_or(BytecodeLowerError::UnknownFunction)?;
        self.push_call_direct(dst, function, args)
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
        self.push_call_direct(dst, function, args)
    }

    pub(super) fn lower_callable_value(
        &mut self,
        dst: Reg,
        callable: ExprId,
        captures: &[Reg],
    ) -> Result<(), BytecodeLowerError> {
        let function = *self
            .callable_indices
            .get(&callable)
            .ok_or(BytecodeLowerError::UnknownFunction)?;
        let site = u32::try_from(self.callable_sites.len())
            .map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.callable_sites.push(BytecodeCallableSite {
            function,
            captures: captures.iter().map(|capture| capture.0).collect(),
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

    fn push_call_direct(
        &mut self,
        dst: Reg,
        function: u32,
        args: &[Reg],
    ) -> Result<(), BytecodeLowerError> {
        let call_site =
            u32::try_from(self.call_sites.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.call_sites.push(BytecodeCallSite {
            function,
            args: args.iter().map(|arg| arg.0).collect(),
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
