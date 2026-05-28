use tune_resolve::NameTarget;
use tune_shape::Shape;

use crate::IrOp;
use crate::lower::{IrLowerError, Lowerer};

impl Lowerer {
    pub(super) fn lower_sequence_build(
        &mut self,
        _element_count: usize,
    ) -> Result<(), IrLowerError> {
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::SeqBuild {
            dst,
            element_shape: Shape::Hole,
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

    pub(super) fn lower_sequence_get(&mut self, checked: bool) -> Result<(), IrLowerError> {
        let index = self.pop("sequence index")?;
        let seq = self.pop("sequence base")?;
        let dst = self.alloc_reg()?;
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
        base_target: Option<NameTarget>,
    ) -> Result<(), IrLowerError> {
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
