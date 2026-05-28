use tune_diagnostics::Span;

use crate::IrOp;
use crate::lower::{IrLowerError, Lowerer};

impl Lowerer {
    pub(super) fn lower_spawn(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let value = self.pop("spawn value")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::Spawn {
            dst,
            callable: value,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_task_join(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let task = self.pop("task join")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::TaskJoin { dst, task, span });
        self.stack.push(dst);
        Ok(())
    }
}
