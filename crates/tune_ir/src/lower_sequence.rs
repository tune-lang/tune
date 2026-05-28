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
}
