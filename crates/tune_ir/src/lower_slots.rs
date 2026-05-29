use tune_hir::HirId;
use tune_plan::CaptureSource;
use tune_resolve::LocalId;

use crate::lower::IrLowerError;

pub(super) fn local_offset(
    module_bindings: &[HirId],
    params: &[tune_hir::MemberId],
    local_params: &[LocalId],
    captures: &[CaptureSource],
) -> u32 {
    let offset = module_bindings
        .len()
        .saturating_add(params.len())
        .saturating_add(local_params.len())
        .saturating_add(captures.len());
    u32::try_from(offset).unwrap_or(u32::MAX)
}

pub(super) fn local_slot(local: LocalId, offset: u32) -> Result<LocalId, IrLowerError> {
    Ok(LocalId(
        local
            .0
            .checked_add(offset)
            .ok_or(IrLowerError::RegisterLimit)?,
    ))
}

pub(super) fn module_slot(item: HirId, module_bindings: &[HirId]) -> Result<LocalId, IrLowerError> {
    let index = module_bindings
        .iter()
        .position(|binding| *binding == item)
        .ok_or(IrLowerError::UnsupportedOp("module binding"))?;
    Ok(LocalId(
        u32::try_from(index).map_err(|_| IrLowerError::RegisterLimit)?,
    ))
}

pub(super) fn param_slot(
    param: tune_hir::MemberId,
    module_bindings: &[HirId],
    params: &[tune_hir::MemberId],
) -> Result<LocalId, IrLowerError> {
    let index = params
        .iter()
        .position(|candidate| *candidate == param)
        .ok_or(IrLowerError::UnsupportedOp("param binding"))?;
    let slot = module_bindings
        .len()
        .checked_add(index)
        .ok_or(IrLowerError::RegisterLimit)?;
    Ok(LocalId(
        u32::try_from(slot).map_err(|_| IrLowerError::RegisterLimit)?,
    ))
}

pub(super) fn local_param_slot(
    param: LocalId,
    module_bindings: &[HirId],
    captures: &[CaptureSource],
    local_params: &[LocalId],
) -> Result<LocalId, IrLowerError> {
    let index = local_params
        .iter()
        .position(|candidate| *candidate == param)
        .ok_or(IrLowerError::UnsupportedOp("callable value param binding"))?;
    let slot = module_bindings
        .len()
        .checked_add(captures.len())
        .and_then(|offset| offset.checked_add(index))
        .ok_or(IrLowerError::RegisterLimit)?;
    Ok(LocalId(
        u32::try_from(slot).map_err(|_| IrLowerError::RegisterLimit)?,
    ))
}

pub(super) fn capture_slot(
    capture: CaptureSource,
    module_bindings: &[HirId],
    captures: &[CaptureSource],
) -> Result<LocalId, IrLowerError> {
    let index = captures
        .iter()
        .position(|candidate| *candidate == capture)
        .ok_or(IrLowerError::UnsupportedOp("capture binding"))?;
    let slot = module_bindings
        .len()
        .checked_add(index)
        .ok_or(IrLowerError::RegisterLimit)?;
    Ok(LocalId(
        u32::try_from(slot).map_err(|_| IrLowerError::RegisterLimit)?,
    ))
}
