use tune_diagnostics::Span;
use tune_hir::MemberId;
use tune_plan::StructStatePlan;
use tune_resolve::{NameTarget, VariantId};

use crate::lower::Lowerer;
use crate::{FieldId, IrLowerError, IrOp, StructField};

impl Lowerer {
    pub(super) fn lower_variant_construct(
        &mut self,
        variant: VariantId,
        arg_count: usize,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(self.pop("variant argument")?);
        }
        args.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::VariantConstruct {
            dst,
            variant,
            args,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_struct_construct(
        &mut self,
        item: tune_hir::HirId,
        state: StructStatePlan,
        fields: &[MemberId],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut values = Vec::with_capacity(fields.len());
        for field in fields.iter().rev() {
            values.push(StructField {
                field: FieldId(field.index),
                value: self.pop("struct field initializer")?,
            });
        }
        values.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::StructConstruct {
            dst,
            item,
            state: crate::lower_state::lower_struct_state(state),
            fields: values,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_field_get(
        &mut self,
        member: Option<MemberId>,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let Some(member) = member else {
            return Err(IrLowerError::UnsupportedOp("unresolved field get"));
        };
        let base = self.pop("field base")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::GetField {
            dst,
            base,
            field: FieldId(member.index),
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_field_set(
        &mut self,
        member: Option<MemberId>,
        base_target: Option<NameTarget>,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let Some(member) = member else {
            return Err(IrLowerError::UnsupportedOp("unresolved field set"));
        };
        let value = self.pop("field value")?;
        let base = self.pop("field base")?;
        self.push_op(IrOp::SetField {
            base,
            field: FieldId(member.index),
            value,
            span,
        });
        if let Some(target) = base_target {
            self.store_binding_target(target, base)?;
        }
        Ok(())
    }
}
