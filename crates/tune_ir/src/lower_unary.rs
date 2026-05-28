use tune_hir::expr::UnaryOp;

use crate::IrOp;
use crate::lower::{IrLowerError, Lowerer};

impl Lowerer {
    pub(super) fn lower_unary(&mut self, op: UnaryOp) -> Result<(), IrLowerError> {
        match op {
            UnaryOp::Neg => self.lower_neg_int(),
            UnaryOp::Not => self.lower_not_bool(),
            UnaryOp::BitNot => Err(IrLowerError::UnsupportedOp("unary op")),
        }
    }

    fn lower_neg_int(&mut self) -> Result<(), IrLowerError> {
        let value = self.pop("unary value")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::NegInt {
            dst,
            value,
            span: None,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_not_bool(&mut self) -> Result<(), IrLowerError> {
        let value = self.pop("unary value")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::NotBool {
            dst,
            value,
            span: None,
        });
        self.stack.push(dst);
        Ok(())
    }
}
