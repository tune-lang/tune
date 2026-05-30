use tune_resolve::NameTarget;
use tune_shape::Shape;

use crate::IrOp;
use crate::lower::{IrLowerError, Lowerer};

impl Lowerer {
    pub(super) fn lower_sequence_build(
        &mut self,
        _element_count: usize,
        element_shape: &Shape,
    ) -> Result<(), IrLowerError> {
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::SeqBuild {
            dst,
            element_shape: element_shape.clone(),
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_sequence_push(&mut self) -> Result<(), IrLowerError> {
        let value = self.pop("sequence element")?;
        let seq = self.pop("sequence value")?;
        self.push_op(IrOp::SeqPush { seq, value });
        self.stack.push(seq);
        Ok(())
    }

    pub(super) fn lower_tuple_build(&mut self, element_count: usize) -> Result<(), IrLowerError> {
        let mut items = Vec::with_capacity(element_count);
        for _ in 0..element_count {
            items.push(self.pop("tuple item")?);
        }
        items.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::TupleBuild { dst, items });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_sequence_get(
        &mut self,
        checked: bool,
        index_member: Option<tune_hir::MemberId>,
    ) -> Result<(), IrLowerError> {
        let index = self.pop("sequence index")?;
        let seq = self.pop("sequence base")?;
        let dst = self.alloc_reg()?;
        if let Some(member) = index_member {
            self.push_op(IrOp::CallMember {
                dst,
                member,
                args: vec![seq, index],
                span: None,
            });
            self.stack.push(dst);
            return Ok(());
        }
        self.push_op(IrOp::SeqGet {
            dst,
            seq,
            index,
            checked,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_sequence_set(
        &mut self,
        checked: bool,
        index_member: Option<tune_hir::MemberId>,
        base_target: Option<NameTarget>,
    ) -> Result<(), IrLowerError> {
        if index_member.is_some() {
            return Err(IrLowerError::UnsupportedOp("index member assignment"));
        }
        let value = self.pop("sequence value")?;
        let index = self.pop("sequence index")?;
        let seq = self.pop("sequence base")?;
        self.push_op(IrOp::SeqSet {
            seq,
            index,
            value,
            checked,
        });
        if let Some(target) = base_target {
            self.store_binding_target(target, seq)?;
        }
        Ok(())
    }
}
