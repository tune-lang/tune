use crate::artifact::BytecodeArtifact;
use crate::function::BytecodeFunction;

use super::BytecodeValidationError;

pub(super) fn register(
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

pub(super) fn local(
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

pub(super) fn jump(
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

pub(super) fn checked_index(index: usize) -> Result<u32, BytecodeValidationError> {
    u32::try_from(index)
        .map_err(|_| BytecodeValidationError::FunctionOutOfBounds { function: u32::MAX })
}

pub(super) fn field_site(
    artifact: &BytecodeArtifact,
    function_id: u32,
    function: &BytecodeFunction,
    site: u32,
) -> Result<(), BytecodeValidationError> {
    let site = function.field_sites.get(site as usize).ok_or(
        BytecodeValidationError::FieldIndexOutOfBounds {
            function: function_id,
            field: site,
        },
    )?;
    validate_struct_field(artifact, function_id, site.owner, site.field)
}

pub(super) fn validate_struct_layout(
    artifact: &BytecodeArtifact,
    function_id: u32,
    owner: u32,
) -> Result<(), BytecodeValidationError> {
    if artifact
        .struct_layouts
        .iter()
        .any(|layout| layout.owner == owner)
    {
        return Ok(());
    }
    Err(BytecodeValidationError::StructLayoutMissing {
        function: function_id,
        owner,
    })
}

pub(super) fn validate_struct_field(
    artifact: &BytecodeArtifact,
    function_id: u32,
    owner: u32,
    field: u32,
) -> Result<(), BytecodeValidationError> {
    validate_struct_layout(artifact, function_id, owner)?;
    if artifact
        .struct_layouts
        .iter()
        .any(|layout| layout.owner == owner && layout.fields.contains(&field))
    {
        return Ok(());
    }
    Err(BytecodeValidationError::FieldIndexOutOfBounds {
        function: function_id,
        field,
    })
}
