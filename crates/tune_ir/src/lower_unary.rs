use tune_hir::expr::UnaryOp;
use tune_shape::Shape;

use crate::lower::{IrLowerError, Lowerer};
use crate::{IrByteBinary, IrOp};

impl Lowerer {
    pub(super) fn lower_unary(&mut self, op: UnaryOp, shape: &Shape) -> Result<(), IrLowerError> {
        match op {
            UnaryOp::Neg => self.lower_neg_int(),
            UnaryOp::Not => self.lower_not_bool(),
            UnaryOp::BitNot if matches!(shape, Shape::Byte) => self.lower_bit_not_byte(),
            UnaryOp::BitNot => self.lower_bit_not_int(),
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

    fn lower_bit_not_int(&mut self) -> Result<(), IrLowerError> {
        let value = self.pop("unary value")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::BitNotInt {
            dst,
            value,
            span: None,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_bit_not_byte(&mut self) -> Result<(), IrLowerError> {
        let value = self.pop("unary value")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::ByteBinary {
            dst,
            a: value,
            b: value,
            op: IrByteBinary::BitNot,
            span: None,
        });
        self.stack.push(dst);
        Ok(())
    }
}
