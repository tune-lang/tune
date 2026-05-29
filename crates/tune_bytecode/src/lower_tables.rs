use std::collections::HashMap;

use crate::artifact::BytecodeConst;
use crate::function::BytecodeVariant;
use crate::lower::BytecodeLowerError;
use tune_hir::{ExprId, HirId, MemberId};
use tune_ir::{BlockId, IrConst, IrFunction, IrOp};
use tune_resolve::{PreludeVariant, VariantId};

pub(super) fn lower_variant(variant: VariantId) -> BytecodeVariant {
    match variant {
        VariantId::Prelude(PreludeVariant::Ok) => BytecodeVariant::ResultOk,
        VariantId::Prelude(PreludeVariant::Error) => BytecodeVariant::ResultError,
        VariantId::Member(member) => BytecodeVariant::Other {
            owner: member.owner.0,
            index: member.index,
        },
    }
}

pub(super) fn push_artifact_const(
    constants: &mut Vec<BytecodeConst>,
    constant: &IrConst,
) -> Result<u32, BytecodeLowerError> {
    let index = u32::try_from(constants.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
    match constant {
        IrConst::Int(value) => constants.push(BytecodeConst::Int(*value)),
        IrConst::Float(value) => constants.push(BytecodeConst::Float(*value)),
        IrConst::Size(value) => constants.push(BytecodeConst::Size(*value)),
        IrConst::Byte(value) => constants.push(BytecodeConst::Byte(*value)),
        IrConst::Bool(value) => constants.push(BytecodeConst::Bool(*value)),
        IrConst::None => constants.push(BytecodeConst::None),
        IrConst::String(value) => constants.push(BytecodeConst::String(value.clone())),
    }
    Ok(index)
}

pub(super) fn block_offsets(
    function: &IrFunction,
) -> Result<HashMap<BlockId, u32>, BytecodeLowerError> {
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

pub(super) fn function_indices(
    functions: &[IrFunction],
) -> Result<HashMap<HirId, u32>, BytecodeLowerError> {
    let mut indices = HashMap::new();
    for (index, function) in functions.iter().enumerate() {
        if function.member.is_none()
            && function.callable.is_none()
            && let Some(owner) = function.owner
        {
            indices.insert(
                owner,
                u32::try_from(index).map_err(|_| BytecodeLowerError::ConstantLimit)?,
            );
        }
    }
    Ok(indices)
}

pub(super) fn member_function_indices(
    functions: &[IrFunction],
) -> Result<HashMap<MemberId, u32>, BytecodeLowerError> {
    let mut indices = HashMap::new();
    for (index, function) in functions.iter().enumerate() {
        if let Some(member) = function.member {
            indices.insert(
                member,
                u32::try_from(index).map_err(|_| BytecodeLowerError::ConstantLimit)?,
            );
        }
    }
    Ok(indices)
}

pub(super) fn callable_function_indices(
    functions: &[IrFunction],
) -> Result<HashMap<ExprId, u32>, BytecodeLowerError> {
    let mut indices = HashMap::new();
    for (index, function) in functions.iter().enumerate() {
        if let Some(callable) = function.callable {
            indices.insert(
                callable,
                u32::try_from(index).map_err(|_| BytecodeLowerError::ConstantLimit)?,
            );
        }
    }
    Ok(indices)
}

fn instruction_count(op: &IrOp) -> u32 {
    match op {
        IrOp::Branch { .. } => 2,
        _ => 1,
    }
}
