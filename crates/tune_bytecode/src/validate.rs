use crate::Opcode;
use crate::artifact::BytecodeArtifact;
use crate::function::{BytecodeFunction, Instruction};

mod generics;
mod sites;
mod support;

use sites::{
    validate_bound_call, validate_call, validate_callable, validate_host_call, validate_match,
    validate_panic, validate_string, validate_struct, validate_task, validate_tuple,
    validate_variant,
};
use support::{checked_index, field_site, jump, local, register, validate_finite_for};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeValidationError {
    MissingEntry,
    EntryOutOfBounds,
    ParamCountExceedsLocals {
        function: u32,
    },
    RegisterOutOfBounds {
        function: u32,
        register: u32,
    },
    LocalOutOfBounds {
        function: u32,
        local: u32,
    },
    ConstantOutOfBounds {
        constant: u32,
    },
    FunctionOutOfBounds {
        function: u32,
    },
    CallSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    BoundCallSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    HostCallSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    CallableSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    TaskSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    StructSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    StructLayoutMissing {
        function: u32,
        owner: u32,
    },
    VariantSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    MatchSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    ForSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    PanicSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    TupleSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    StringSiteOutOfBounds {
        function: u32,
        site: u32,
    },
    FieldIndexOutOfBounds {
        function: u32,
        field: u32,
    },
    JumpOutOfBounds {
        function: u32,
        target: u32,
    },
    ProvenanceLengthMismatch {
        function: u32,
        instructions: u32,
        spans: u32,
    },
    FrameLayoutMismatch {
        function: u32,
    },
    CallArityMismatch {
        function: u32,
        target: u32,
        expected: u32,
        actual: u32,
    },
    GenericArgArityMismatch {
        function: u32,
        target: u32,
        expected: u32,
        actual: u32,
    },
    UnsolvedGenericArg {
        function: u32,
        target: u32,
    },
    GenericStrategyMismatch {
        function: u32,
        target: u32,
    },
}

pub fn validate_artifact(artifact: &BytecodeArtifact) -> Result<(), BytecodeValidationError> {
    let entry = artifact
        .entry_function
        .ok_or(BytecodeValidationError::MissingEntry)?;
    if entry as usize >= artifact.functions.len() {
        return Err(BytecodeValidationError::EntryOutOfBounds);
    }

    for (index, function) in artifact.functions.iter().enumerate() {
        validate_function(artifact, checked_index(index)?, function)?;
    }
    Ok(())
}

fn validate_function(
    artifact: &BytecodeArtifact,
    function_id: u32,
    function: &BytecodeFunction,
) -> Result<(), BytecodeValidationError> {
    if function.param_count > function.local_count {
        return Err(BytecodeValidationError::ParamCountExceedsLocals {
            function: function_id,
        });
    }
    if function.frame.params.len() != function.param_count as usize
        || function.frame.locals.len() != function.local_count as usize
        || function.frame.registers.len() != function.register_count as usize
    {
        return Err(BytecodeValidationError::FrameLayoutMismatch {
            function: function_id,
        });
    }
    let spans = checked_index(function.provenance.instruction_spans.len())?;
    let instructions = checked_index(function.instructions.len())?;
    if spans != 0 && spans != instructions {
        return Err(BytecodeValidationError::ProvenanceLengthMismatch {
            function: function_id,
            instructions,
            spans,
        });
    }
    for instruction in &function.instructions {
        validate_instruction(artifact, function_id, function, instruction)?;
    }
    Ok(())
}

fn validate_instruction(
    artifact: &BytecodeArtifact,
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    match instruction.opcode {
        Opcode::LoadConst => {
            register(function_id, function, instruction.a)?;
            if instruction.b as usize >= artifact.constants.len() {
                return Err(BytecodeValidationError::ConstantOutOfBounds {
                    constant: instruction.b,
                });
            }
        }
        Opcode::LoadLocal => {
            register(function_id, function, instruction.a)?;
            local(function_id, function, instruction.b)?;
        }
        Opcode::StoreLocal => {
            local(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::Move => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::SeqBuild => {
            register(function_id, function, instruction.a)?;
        }
        Opcode::SeqPush | Opcode::SeqPushExclusive | Opcode::SeqPushShared => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::SeqGetChecked
        | Opcode::SeqGetUnchecked
        | Opcode::SeqSetChecked
        | Opcode::SeqSetUnchecked
        | Opcode::SeqSetCheckedExclusive
        | Opcode::SeqSetUncheckedExclusive
        | Opcode::SeqSetCheckedShared
        | Opcode::SeqSetUncheckedShared => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
            register(function_id, function, instruction.c)?;
        }
        Opcode::NegInt
        | Opcode::NotBool
        | Opcode::BitNotInt
        | Opcode::BitNotSize
        | Opcode::NoneCheck => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::AddInt
        | Opcode::SubInt
        | Opcode::MulInt
        | Opcode::DivInt
        | Opcode::RemInt
        | Opcode::BitAndInt
        | Opcode::BitOrInt
        | Opcode::BitXorInt
        | Opcode::ShiftLeftInt
        | Opcode::ShiftRightInt
        | Opcode::AddFloat
        | Opcode::SubFloat
        | Opcode::MulFloat
        | Opcode::DivFloat
        | Opcode::AddSizeChecked
        | Opcode::SubSizeChecked
        | Opcode::MulSizeChecked
        | Opcode::DivSize
        | Opcode::RemSize
        | Opcode::BitAndSize
        | Opcode::BitOrSize
        | Opcode::BitXorSize
        | Opcode::ShiftLeftSize
        | Opcode::ShiftRightSize
        | Opcode::AddByteWrap
        | Opcode::RangeExclusiveInt
        | Opcode::RangeInclusiveInt
        | Opcode::GreaterInt
        | Opcode::EqualInt
        | Opcode::NotEqualInt
        | Opcode::LessInt
        | Opcode::LessEqualInt
        | Opcode::GreaterEqualInt
        | Opcode::GreaterFloat
        | Opcode::EqualFloat
        | Opcode::NotEqualFloat
        | Opcode::LessFloat
        | Opcode::LessEqualFloat
        | Opcode::GreaterEqualFloat
        | Opcode::GreaterSize
        | Opcode::EqualSize
        | Opcode::NotEqualSize
        | Opcode::LessSize
        | Opcode::LessEqualSize
        | Opcode::GreaterEqualSize
        | Opcode::SubByteWrap
        | Opcode::MulByteWrap
        | Opcode::DivByte
        | Opcode::RemByte
        | Opcode::BitNotByte
        | Opcode::BitAndByte
        | Opcode::BitOrByte
        | Opcode::BitXorByte
        | Opcode::ShiftLeftByte
        | Opcode::ShiftRightByte
        | Opcode::GreaterByte
        | Opcode::EqualByte
        | Opcode::NotEqualByte
        | Opcode::LessByte
        | Opcode::LessEqualByte
        | Opcode::GreaterEqualByte => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
            register(function_id, function, instruction.c)?;
        }
        Opcode::StructConstruct => validate_struct(artifact, function_id, function, instruction)?,
        Opcode::StructIs => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::FieldGet => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
            field_site(artifact, function_id, function, instruction.c)?;
        }
        Opcode::FieldSet => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.c)?;
            field_site(artifact, function_id, function, instruction.b)?;
        }
        Opcode::CallDirect => validate_call(artifact, function_id, function, instruction)?,
        Opcode::CallBound => validate_bound_call(function_id, function, instruction)?,
        Opcode::CallHost => validate_host_call(function_id, function, instruction)?,
        Opcode::TupleBuild => validate_tuple(function_id, function, instruction)?,
        Opcode::StringBuild => validate_string(function_id, function, instruction)?,
        Opcode::StringLen | Opcode::SeqLen => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::StringGet => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
            register(function_id, function, instruction.c)?;
        }
        Opcode::CallableValue => validate_callable(artifact, function_id, function, instruction)?,
        Opcode::VariantConstruct => validate_variant(function_id, function, instruction)?,
        Opcode::VariantField | Opcode::TupleField | Opcode::ResultPropagate | Opcode::TaskJoin => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::SpawnTask => validate_task(artifact, function_id, function, instruction)?,
        Opcode::Jump => jump(function_id, function, instruction.a)?,
        Opcode::JumpIfFalse => {
            register(function_id, function, instruction.a)?;
            jump(function_id, function, instruction.b)?;
        }
        Opcode::MatchVariant => validate_match(function_id, function, instruction)?,
        Opcode::FiniteForInit => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
            register(function_id, function, instruction.c)?;
        }
        Opcode::FiniteForNext => validate_finite_for(function_id, function, instruction)?,
        Opcode::Panic => validate_panic(function_id, function, instruction)?,
        Opcode::Return if instruction.b != 0 => {
            register(function_id, function, instruction.a)?;
        }
        Opcode::Return => {}
        Opcode::Nop => {}
        _ => {}
    }
    Ok(())
}
