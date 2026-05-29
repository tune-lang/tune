use tune_plan::CaptureSource;
use tune_resolve::NameTarget;

use crate::lower::Lowerer;
use crate::lower_slots::{
    capture_slot, local_offset, local_param_slot, local_slot, module_slot, param_slot,
};
use crate::{IrLowerError, IrOp};

impl Lowerer {
    pub(super) fn lower_binding_get(&mut self, source: NameTarget) -> Result<(), IrLowerError> {
        let local = match source {
            NameTarget::Local(local) if self.captures.contains(&CaptureSource::Local(local)) => {
                capture_slot(
                    CaptureSource::Local(local),
                    &self.module_bindings,
                    &self.captures,
                )?
            }
            NameTarget::Local(local) if self.local_params.contains(&local) => local_param_slot(
                local,
                &self.module_bindings,
                &self.captures,
                &self.local_params,
            )?,
            NameTarget::Local(local) => local_slot(
                local,
                local_offset(
                    &self.module_bindings,
                    &self.params,
                    &self.local_params,
                    &self.captures,
                ),
            )?,
            NameTarget::Param(param) => param_slot(param, &self.module_bindings, &self.params)?,
            NameTarget::SelfValue => tune_resolve::LocalId(0),
            NameTarget::TopLevel(item)
                if self.captures.contains(&CaptureSource::TopLevel(item)) =>
            {
                capture_slot(
                    CaptureSource::TopLevel(item),
                    &self.module_bindings,
                    &self.captures,
                )?
            }
            NameTarget::TopLevel(item) if self.module_bindings.contains(&item) => {
                module_slot(item, &self.module_bindings)?
            }
            NameTarget::TopLevel(_) | NameTarget::Variant(_) => {
                return Err(IrLowerError::UnsupportedOp("binding get"));
            }
        };
        self.track_local(local)?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::LoadLocal { dst, local });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_local_let(
        &mut self,
        local: Option<tune_resolve::LocalId>,
        initialized: bool,
    ) -> Result<(), IrLowerError> {
        if !initialized {
            return Ok(());
        }
        let Some(local) = local else {
            return Err(IrLowerError::UnsupportedOp("unresolved local initializer"));
        };
        let local = local_slot(
            local,
            local_offset(
                &self.module_bindings,
                &self.params,
                &self.local_params,
                &self.captures,
            ),
        )?;
        self.track_local(local)?;
        let value = self.pop("local initializer")?;
        self.push_op(IrOp::StoreLocal { local, value });
        Ok(())
    }

    pub(super) fn lower_module_let(
        &mut self,
        item: tune_hir::HirId,
        initialized: bool,
        keep_value: bool,
    ) -> Result<(), IrLowerError> {
        if !initialized {
            return Ok(());
        }
        let local = module_slot(item, &self.module_bindings)?;
        self.track_local(local)?;
        let value = self.pop("module initializer")?;
        self.push_op(IrOp::StoreLocal { local, value });
        if keep_value {
            self.stack.push(value);
        }
        Ok(())
    }

    pub(super) fn lower_binding_set(
        &mut self,
        target: Option<NameTarget>,
    ) -> Result<(), IrLowerError> {
        let Some(NameTarget::Local(local)) = target else {
            return Err(IrLowerError::UnsupportedOp("binding set"));
        };
        let local = self.lower_local_source_slot(local)?;
        self.track_local(local)?;
        let value = self.pop("local assignment")?;
        self.push_op(IrOp::StoreLocal { local, value });
        Ok(())
    }

    pub(super) fn store_binding_target(
        &mut self,
        target: NameTarget,
        value: crate::Reg,
    ) -> Result<(), IrLowerError> {
        match target {
            NameTarget::Local(local) => {
                let local = self.lower_local_source_slot(local)?;
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::Param(param) => {
                let local = param_slot(param, &self.module_bindings, &self.params)?;
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::TopLevel(item) if self.module_bindings.contains(&item) => {
                let local = module_slot(item, &self.module_bindings)?;
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::SelfValue => {
                let local = tune_resolve::LocalId(0);
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::TopLevel(_) | NameTarget::Variant(_) => Ok(()),
        }
    }

    fn lower_local_source_slot(
        &self,
        local: tune_resolve::LocalId,
    ) -> Result<tune_resolve::LocalId, IrLowerError> {
        if self.captures.contains(&CaptureSource::Local(local)) {
            return capture_slot(
                CaptureSource::Local(local),
                &self.module_bindings,
                &self.captures,
            );
        }
        if self.local_params.contains(&local) {
            return local_param_slot(
                local,
                &self.module_bindings,
                &self.captures,
                &self.local_params,
            );
        }
        local_slot(
            local,
            local_offset(
                &self.module_bindings,
                &self.params,
                &self.local_params,
                &self.captures,
            ),
        )
    }
}
