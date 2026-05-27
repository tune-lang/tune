use std::collections::HashMap;

use crate::Opcode;
use crate::artifact::{BytecodeArtifact, BytecodeConst};
use crate::function::{
    BytecodeCallSite, BytecodeFunction, BytecodeVariant, BytecodeVariantSite, Instruction,
};
use tune_hir::HirId;
use tune_ir::{BlockId, IrConst, IrFunction, IrOp};
use tune_resolve::{PreludeVariant, VariantId};

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
    let mut constants = Vec::new();
    let functions = functions
        .iter()
        .map(|function| {
            lower_ir_function_with_constants(function, &function_indices, &mut constants)
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
    lower_ir_function_with_constants(function, &function_indices, &mut constants)
}

fn lower_ir_function_with_constants(
    function: &IrFunction,
    function_indices: &HashMap<HirId, u32>,
    constants: &mut Vec<BytecodeConst>,
) -> Result<BytecodeFunction, BytecodeLowerError> {
    let block_offsets = block_offsets(function)?;
    let mut lowerer = FunctionLowerer {
        function,
        function_indices,
        block_offsets,
        constants,
        call_sites: Vec::new(),
        variant_sites: Vec::new(),
        instructions: Vec::new(),
    };
    for block in &function.blocks {
        for op in &block.ops {
            lowerer.lower_op(op)?;
        }
    }
    Ok(BytecodeFunction {
        name: function.name.clone(),
        register_count: function.regs,
        local_count: function.locals,
        call_sites: lowerer.call_sites,
        variant_sites: lowerer.variant_sites,
        instructions: lowerer.instructions,
    })
}

struct FunctionLowerer<'a> {
    function: &'a IrFunction,
    function_indices: &'a HashMap<HirId, u32>,
    block_offsets: HashMap<BlockId, u32>,
    constants: &'a mut Vec<BytecodeConst>,
    call_sites: Vec<BytecodeCallSite>,
    variant_sites: Vec<BytecodeVariantSite>,
    instructions: Vec<Instruction>,
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
            IrOp::ResultPropagate { dst, result, .. } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::ResultPropagate,
                    a: dst.0,
                    b: result.0,
                    c: 0,
                });
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

fn lower_variant(variant: VariantId) -> BytecodeVariant {
    match variant {
        VariantId::Prelude(PreludeVariant::Ok) => BytecodeVariant::ResultOk,
        VariantId::Prelude(PreludeVariant::Error) => BytecodeVariant::ResultError,
        VariantId::Member(member) => BytecodeVariant::Other(member.index),
    }
}

fn push_artifact_const(
    constants: &mut Vec<BytecodeConst>,
    constant: &IrConst,
) -> Result<u32, BytecodeLowerError> {
    let index = u32::try_from(constants.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
    match constant {
        IrConst::Int(value) => constants.push(BytecodeConst::Int(*value)),
        IrConst::Bool(value) => constants.push(BytecodeConst::Bool(*value)),
    }
    Ok(index)
}

fn block_offsets(function: &IrFunction) -> Result<HashMap<BlockId, u32>, BytecodeLowerError> {
    let mut offsets = HashMap::new();
    let mut offset = 0_u32;
    for block in &function.blocks {
        offsets.insert(block.id, offset);
        for op in &block.ops {
            offset = offset
                .checked_add(instruction_count(op))
                .ok_or(BytecodeLowerError::ConstantLimit)?;
        }
    }
    Ok(offsets)
}

fn instruction_count(op: &IrOp) -> u32 {
    match op {
        IrOp::Branch { .. } => 2,
        _ => 1,
    }
}

fn function_indices(functions: &[IrFunction]) -> Result<HashMap<HirId, u32>, BytecodeLowerError> {
    let mut indices = HashMap::new();
    for (index, function) in functions.iter().enumerate() {
        if let Some(owner) = function.owner {
            indices.insert(
                owner,
                u32::try_from(index).map_err(|_| BytecodeLowerError::ConstantLimit)?,
            );
        }
    }
    Ok(indices)
}
