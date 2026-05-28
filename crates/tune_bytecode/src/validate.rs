use crate::Opcode;
use crate::artifact::BytecodeArtifact;
use crate::function::{BytecodeFunction, Instruction};

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
    StructSiteOutOfBounds {
        function: u32,
        site: u32,
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
    JumpOutOfBounds {
        function: u32,
        target: u32,
    },
    ProvenanceLengthMismatch {
        function: u32,
        instructions: u32,
        spans: u32,
    },
    CallArityMismatch {
        function: u32,
        target: u32,
        expected: u32,
        actual: u32,
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
        Opcode::SeqPush => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::NegInt | Opcode::NotBool => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::AddInt
        | Opcode::GreaterInt
        | Opcode::EqualInt
        | Opcode::NotEqualInt
        | Opcode::LessInt
        | Opcode::LessEqualInt
        | Opcode::GreaterEqualInt => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
            register(function_id, function, instruction.c)?;
        }
        Opcode::StructConstruct => validate_struct(function_id, function, instruction)?,
        Opcode::FieldGet => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
        Opcode::FieldSet => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.c)?;
        }
        Opcode::CallDirect => validate_call(artifact, function_id, function, instruction)?,
        Opcode::VariantConstruct => validate_variant(function_id, function, instruction)?,
        Opcode::VariantField | Opcode::ResultPropagate | Opcode::SpawnTask | Opcode::TaskJoin => {
            register(function_id, function, instruction.a)?;
            register(function_id, function, instruction.b)?;
        }
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
        Opcode::Return if instruction.b != 0 => {
            register(function_id, function, instruction.a)?;
        }
        Opcode::Return => {}
        Opcode::Nop => {}
        _ => {}
    }
    Ok(())
}

fn validate_struct(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.struct_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::StructSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    for field in &site.fields {
        register(function_id, function, field.value)?;
    }
    Ok(())
}

fn validate_call(
    artifact: &BytecodeArtifact,
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.call_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::CallSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    let target = artifact.functions.get(site.function as usize).ok_or(
        BytecodeValidationError::FunctionOutOfBounds {
            function: site.function,
        },
    )?;
    let actual =
        u32::try_from(site.args.len()).map_err(|_| BytecodeValidationError::CallArityMismatch {
            function: function_id,
            target: site.function,
            expected: target.param_count,
            actual: u32::MAX,
        })?;
    if actual != target.param_count {
        return Err(BytecodeValidationError::CallArityMismatch {
            function: function_id,
            target: site.function,
            expected: target.param_count,
            actual,
        });
    }
    for arg in &site.args {
        register(function_id, function, *arg)?;
    }
    Ok(())
}

fn validate_variant(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.variant_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::VariantSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    for arg in &site.args {
        register(function_id, function, *arg)?;
    }
    Ok(())
}

fn validate_match(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.match_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::MatchSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    for arm in &site.arms {
        jump(function_id, function, arm.target)?;
    }
    if instruction.c != u32::MAX {
        jump(function_id, function, instruction.c)?;
    }
    Ok(())
}

fn validate_finite_for(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.for_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::ForSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    register(function_id, function, site.iterable)?;
    register(function_id, function, site.len)?;
    register(function_id, function, site.index)?;
    register(function_id, function, site.item)?;
    jump(function_id, function, site.body)?;
    jump(function_id, function, site.done)?;
    Ok(())
}

fn register(
    function_id: u32,
    function: &BytecodeFunction,
    register: u32,
) -> Result<(), BytecodeValidationError> {
    if register >= function.register_count {
        return Err(BytecodeValidationError::RegisterOutOfBounds {
            function: function_id,
            register,
        });
    }
    Ok(())
}

fn local(
    function_id: u32,
    function: &BytecodeFunction,
    local: u32,
) -> Result<(), BytecodeValidationError> {
    if local >= function.local_count {
        return Err(BytecodeValidationError::LocalOutOfBounds {
            function: function_id,
            local,
        });
    }
    Ok(())
}

fn jump(
    function_id: u32,
    function: &BytecodeFunction,
    target: u32,
) -> Result<(), BytecodeValidationError> {
    if target as usize >= function.instructions.len() {
        return Err(BytecodeValidationError::JumpOutOfBounds {
            function: function_id,
            target,
        });
    }
    Ok(())
}

fn checked_index(index: usize) -> Result<u32, BytecodeValidationError> {
    u32::try_from(index)
        .map_err(|_| BytecodeValidationError::FunctionOutOfBounds { function: u32::MAX })
}
