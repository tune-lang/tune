use crate::Opcode;
use crate::artifact::{BytecodeArtifact, BytecodeConst};
use crate::function::{BytecodeFunction, Instruction};
use tune_ir::{IrConst, IrFunction, IrOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeLowerError {
    UnsupportedIr(&'static str),
    ConstantLimit,
}

pub fn lower_ir_functions(
    functions: &[IrFunction],
) -> Result<BytecodeArtifact, BytecodeLowerError> {
    let mut constants = Vec::new();
    let functions = functions
        .iter()
        .map(|function| lower_ir_function_with_constants(function, &mut constants))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BytecodeArtifact {
        entry_function: (!functions.is_empty()).then_some(0),
        functions,
        constants,
    })
}

pub fn lower_ir_function(function: &IrFunction) -> Result<BytecodeFunction, BytecodeLowerError> {
    let mut constants = Vec::new();
    lower_ir_function_with_constants(function, &mut constants)
}

fn lower_ir_function_with_constants(
    function: &IrFunction,
    constants: &mut Vec<BytecodeConst>,
) -> Result<BytecodeFunction, BytecodeLowerError> {
    let mut instructions = Vec::new();
    for block in &function.blocks {
        for op in &block.ops {
            lower_op(op, function, constants, &mut instructions)?;
        }
    }
    Ok(BytecodeFunction {
        name: function.name.clone(),
        register_count: function.regs,
        local_count: function.locals,
        instructions,
    })
}

fn lower_op(
    op: &IrOp,
    function: &IrFunction,
    constants: &mut Vec<BytecodeConst>,
    instructions: &mut Vec<Instruction>,
) -> Result<(), BytecodeLowerError> {
    match op {
        IrOp::LoadConst { dst, constant, .. } => {
            let constant = function
                .constants
                .get(constant.0 as usize)
                .ok_or(BytecodeLowerError::ConstantLimit)?;
            let artifact_const = push_artifact_const(constants, constant)?;
            instructions.push(Instruction {
                opcode: Opcode::LoadConst,
                a: dst.0,
                b: artifact_const,
                c: 0,
            });
            Ok(())
        }
        IrOp::AddInt { dst, a, b } => {
            instructions.push(Instruction {
                opcode: Opcode::AddInt,
                a: dst.0,
                b: a.0,
                c: b.0,
            });
            Ok(())
        }
        IrOp::LoadLocal { dst, local } => {
            instructions.push(Instruction {
                opcode: Opcode::LoadLocal,
                a: dst.0,
                b: local.0,
                c: 0,
            });
            Ok(())
        }
        IrOp::StoreLocal { local, value } => {
            instructions.push(Instruction {
                opcode: Opcode::StoreLocal,
                a: local.0,
                b: value.0,
                c: 0,
            });
            Ok(())
        }
        IrOp::Return { value: Some(value) } => {
            instructions.push(Instruction {
                opcode: Opcode::Return,
                a: value.0,
                b: 1,
                c: 0,
            });
            Ok(())
        }
        IrOp::Return { value: None } => {
            instructions.push(Instruction {
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

fn push_artifact_const(
    constants: &mut Vec<BytecodeConst>,
    constant: &IrConst,
) -> Result<u32, BytecodeLowerError> {
    let index = u32::try_from(constants.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
    match constant {
        IrConst::Int(value) => constants.push(BytecodeConst::Int(*value)),
    }
    Ok(index)
}
