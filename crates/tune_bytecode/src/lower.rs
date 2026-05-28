use std::collections::HashMap;

use crate::Opcode;
use crate::artifact::{BytecodeArtifact, BytecodeConst};
use crate::function::{
    BytecodeCallSite, BytecodeFunction, BytecodeMatchArm, BytecodeMatchSite, BytecodeStructField,
    BytecodeStructSite, BytecodeVariantSite, Instruction,
};
use crate::lower_tables::{
    block_offsets, function_indices, lower_variant, member_function_indices, push_artifact_const,
};
use crate::provenance::BytecodeFunctionProvenance;
use tune_hir::{HirId, MemberId};
use tune_ir::{BlockId, IrFunction, IrOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeLowerError {
    UnsupportedIr(&'static str),
    UnknownFunction,
    UnknownBlock,
    ConstantLimit,
}

pub fn lower_ir_functions(
    functions: &[IrFunction],
) -> Result<BytecodeArtifact, BytecodeLowerError> {
    let function_indices = function_indices(functions)?;
    let member_indices = member_function_indices(functions)?;
    let mut constants = Vec::new();
    let functions = functions
        .iter()
        .map(|function| {
            lower_ir_function_with_constants(
                function,
                &function_indices,
                &member_indices,
                &mut constants,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BytecodeArtifact {
        entry_function: (!functions.is_empty()).then_some(0),
        functions,
        constants,
    })
}

pub fn lower_ir_function(function: &IrFunction) -> Result<BytecodeFunction, BytecodeLowerError> {
    let mut constants = Vec::new();
    let function_indices = function_indices(std::slice::from_ref(function))?;
    let member_indices = member_function_indices(std::slice::from_ref(function))?;
    lower_ir_function_with_constants(function, &function_indices, &member_indices, &mut constants)
}

fn lower_ir_function_with_constants(
    function: &IrFunction,
    function_indices: &HashMap<HirId, u32>,
    member_indices: &HashMap<MemberId, u32>,
    constants: &mut Vec<BytecodeConst>,
) -> Result<BytecodeFunction, BytecodeLowerError> {
    let block_offsets = block_offsets(function)?;
    let mut lowerer = FunctionLowerer {
        function,
        function_indices,
        member_indices,
        block_offsets,
        constants,
        call_sites: Vec::new(),
        struct_sites: Vec::new(),
        variant_sites: Vec::new(),
        match_sites: Vec::new(),
        instructions: Vec::new(),
        instruction_spans: Vec::new(),
    };
    for block in &function.blocks {
        for op in &block.ops {
            lowerer.lower_op(op)?;
            lowerer
                .instruction_spans
                .resize(lowerer.instructions.len(), op.provenance_span());
        }
    }
    Ok(BytecodeFunction {
        name: function.name.clone(),
        provenance: BytecodeFunctionProvenance {
            span: function.span,
            instruction_spans: lowerer.instruction_spans,
        },
        param_count: function.params,
        register_count: function.regs,
        local_count: function.locals,
        call_sites: lowerer.call_sites,
        struct_sites: lowerer.struct_sites,
        variant_sites: lowerer.variant_sites,
        match_sites: lowerer.match_sites,
        instructions: lowerer.instructions,
    })
}

pub(super) struct FunctionLowerer<'a> {
    function: &'a IrFunction,
    function_indices: &'a HashMap<HirId, u32>,
    member_indices: &'a HashMap<MemberId, u32>,
    block_offsets: HashMap<BlockId, u32>,
    constants: &'a mut Vec<BytecodeConst>,
    call_sites: Vec<BytecodeCallSite>,
    struct_sites: Vec<BytecodeStructSite>,
    variant_sites: Vec<BytecodeVariantSite>,
    match_sites: Vec<BytecodeMatchSite>,
    pub(super) instructions: Vec<Instruction>,
    instruction_spans: Vec<Option<tune_diagnostics::Span>>,
}

impl FunctionLowerer<'_> {
    fn lower_op(&mut self, op: &IrOp) -> Result<(), BytecodeLowerError> {
        match op {
            IrOp::LoadConst { dst, constant, .. } => {
                let constant = self
                    .function
                    .constants
                    .get(constant.0 as usize)
                    .ok_or(BytecodeLowerError::ConstantLimit)?;
                let artifact_const = push_artifact_const(self.constants, constant)?;
                self.instructions.push(Instruction {
                    opcode: Opcode::LoadConst,
                    a: dst.0,
                    b: artifact_const,
                    c: 0,
                });
                Ok(())
            }
            IrOp::AddInt { dst, a, b } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::AddInt,
                    a: dst.0,
                    b: a.0,
                    c: b.0,
                });
                Ok(())
            }
            IrOp::GreaterInt { dst, a, b } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::GreaterInt,
                    a: dst.0,
                    b: a.0,
                    c: b.0,
                });
                Ok(())
            }
            IrOp::Move { dst, src } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::Move,
                    a: dst.0,
                    b: src.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::LoadLocal { dst, local } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::LoadLocal,
                    a: dst.0,
                    b: local.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::StoreLocal { local, value } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::StoreLocal,
                    a: local.0,
                    b: value.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::GetField { dst, base, field } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::FieldGet,
                    a: dst.0,
                    b: base.0,
                    c: field.0,
                });
                Ok(())
            }
            IrOp::SetField { base, field, value } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::FieldSet,
                    a: base.0,
                    b: field.0,
                    c: value.0,
                });
                Ok(())
            }
            IrOp::CallDirect {
                dst,
                function,
                args,
            } => {
                let function = *self
                    .function_indices
                    .get(function)
                    .ok_or(BytecodeLowerError::UnknownFunction)?;
                let call_site = u32::try_from(self.call_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
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
            IrOp::CallMember { dst, member, args } => {
                let function = *self
                    .member_indices
                    .get(member)
                    .ok_or(BytecodeLowerError::UnknownFunction)?;
                let call_site = u32::try_from(self.call_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
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
            IrOp::VariantConstruct { dst, variant, args } => {
                let variant_site = u32::try_from(self.variant_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.variant_sites.push(BytecodeVariantSite {
                    variant: lower_variant(*variant),
                    args: args.iter().map(|arg| arg.0).collect(),
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::VariantConstruct,
                    a: dst.0,
                    b: variant_site,
                    c: 0,
                });
                Ok(())
            }
            IrOp::StructConstruct {
                dst,
                item,
                state,
                fields,
            } => {
                let site = u32::try_from(self.struct_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.struct_sites.push(BytecodeStructSite {
                    owner: item.0,
                    state: crate::lower_state::lower_struct_state(*state),
                    fields: fields
                        .iter()
                        .map(|field| BytecodeStructField {
                            field: field.field.0,
                            value: field.value.0,
                        })
                        .collect(),
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::StructConstruct,
                    a: dst.0,
                    b: site,
                    c: 0,
                });
                Ok(())
            }
            IrOp::VariantField { dst, base, index } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::VariantField,
                    a: dst.0,
                    b: base.0,
                    c: *index,
                });
                Ok(())
            }
            IrOp::ResultPropagate { dst, result, .. } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::ResultPropagate,
                    a: dst.0,
                    b: result.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::Spawn { dst, callable } => {
                self.lower_spawn(*dst, *callable);
                Ok(())
            }
            IrOp::TaskJoin { dst, task } => {
                self.lower_task_join(*dst, *task);
                Ok(())
            }
            IrOp::Jump { target } => {
                let target = *self
                    .block_offsets
                    .get(target)
                    .ok_or(BytecodeLowerError::UnknownBlock)?;
                self.instructions.push(Instruction {
                    opcode: Opcode::Jump,
                    a: target,
                    b: 0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::Branch {
                condition,
                then_block,
                else_block,
            } => {
                let then_block = *self
                    .block_offsets
                    .get(then_block)
                    .ok_or(BytecodeLowerError::UnknownBlock)?;
                let else_block = *self
                    .block_offsets
                    .get(else_block)
                    .ok_or(BytecodeLowerError::UnknownBlock)?;
                self.instructions.push(Instruction {
                    opcode: Opcode::JumpIfFalse,
                    a: condition.0,
                    b: else_block,
                    c: 0,
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::Jump,
                    a: then_block,
                    b: 0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::MatchVariant {
                scrutinee,
                arms,
                else_block,
            } => {
                let match_site = u32::try_from(self.match_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                let arms = arms
                    .iter()
                    .map(|arm| {
                        Ok(BytecodeMatchArm {
                            variant: lower_variant(arm.variant),
                            target: *self
                                .block_offsets
                                .get(&arm.block)
                                .ok_or(BytecodeLowerError::UnknownBlock)?,
                        })
                    })
                    .collect::<Result<Vec<_>, BytecodeLowerError>>()?;
                let else_target = if let Some(else_block) = else_block {
                    *self
                        .block_offsets
                        .get(else_block)
                        .ok_or(BytecodeLowerError::UnknownBlock)?
                } else {
                    u32::MAX
                };
                self.match_sites.push(BytecodeMatchSite { arms });
                self.instructions.push(Instruction {
                    opcode: Opcode::MatchVariant,
                    a: scrutinee.0,
                    b: match_site,
                    c: else_target,
                });
                Ok(())
            }
            IrOp::Return { value: Some(value) } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::Return,
                    a: value.0,
                    b: 1,
                    c: 0,
                });
                Ok(())
            }
            IrOp::Return { value: None } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::Return,
                    a: 0,
                    b: 0,
                    c: 0,
                });
                Ok(())
            }
            _ => Err(BytecodeLowerError::UnsupportedIr("ir op")),
        }
    }
}
