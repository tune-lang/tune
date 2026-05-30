use crate::artifact::BytecodeArtifact;
use crate::function::{BytecodeFunction, Instruction};
use crate::validate::support::{jump, register, validate_struct_field, validate_struct_layout};
use crate::validate::{BytecodeValidationError, generics};

pub(super) fn validate_bound_call(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    register(function_id, function, instruction.c)?;
    let site = function
        .bound_call_sites
        .get(instruction.b as usize)
        .ok_or(BytecodeValidationError::BoundCallSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        })?;
    for arg in &site.args {
        register(function_id, function, *arg)?;
    }
    Ok(())
}

pub(super) fn validate_host_call(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.host_call_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::HostCallSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    for arg in &site.args {
        register(function_id, function, *arg)?;
    }
    Ok(())
}

pub(super) fn validate_tuple(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.tuple_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::TupleSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    for item in &site.items {
        register(function_id, function, *item)?;
    }
    Ok(())
}

pub(super) fn validate_string(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.string_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::StringSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    for part in &site.parts {
        register(function_id, function, *part)?;
    }
    Ok(())
}

pub(super) fn validate_task(
    artifact: &BytecodeArtifact,
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.task_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::TaskSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    let target = artifact.functions.get(site.function as usize).ok_or(
        BytecodeValidationError::FunctionOutOfBounds {
            function: site.function,
        },
    )?;
    let actual = u32::try_from(site.captures.len()).unwrap_or(u32::MAX);
    if target.param_count != actual {
        return Err(BytecodeValidationError::CallArityMismatch {
            function: function_id,
            target: site.function,
            expected: target.param_count,
            actual,
        });
    }
    for capture in &site.captures {
        register(function_id, function, capture.register)?;
    }
    Ok(())
}

pub(super) fn validate_callable(
    artifact: &BytecodeArtifact,
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    register(function_id, function, instruction.a)?;
    let site = function.callable_sites.get(instruction.b as usize).ok_or(
        BytecodeValidationError::CallableSiteOutOfBounds {
            function: function_id,
            site: instruction.b,
        },
    )?;
    if site.function as usize >= artifact.functions.len() {
        return Err(BytecodeValidationError::FunctionOutOfBounds {
            function: site.function,
        });
    }
    for capture in &site.captures {
        register(function_id, function, capture.register)?;
    }
    Ok(())
}

pub(super) fn validate_struct(
    artifact: &BytecodeArtifact,
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
    validate_struct_layout(artifact, function_id, site.owner)?;
    for field in &site.fields {
        register(function_id, function, field.value)?;
        validate_struct_field(artifact, function_id, site.owner, field.field)?;
    }
    Ok(())
}

pub(super) fn validate_call(
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
    generics::validate_call_generics(function_id, site, target)?;
    for arg in &site.args {
        register(function_id, function, *arg)?;
    }
    Ok(())
}

pub(super) fn validate_variant(
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

pub(super) fn validate_match(
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

pub(super) fn validate_panic(
    function_id: u32,
    function: &BytecodeFunction,
    instruction: &Instruction,
) -> Result<(), BytecodeValidationError> {
    let site = function.panic_sites.get(instruction.a as usize).ok_or(
        BytecodeValidationError::PanicSiteOutOfBounds {
            function: function_id,
            site: instruction.a,
        },
    )?;
    for arg in &site.args {
        register(function_id, function, *arg)?;
    }
    Ok(())
}
