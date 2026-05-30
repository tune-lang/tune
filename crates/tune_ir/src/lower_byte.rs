use tune_diagnostics::Span;

use crate::lower::{IrLowerError, Lowerer};
use crate::lower_binary::{Arithmetic, IntArithmetic};
use crate::{IrByteBinary, IrIntComparison, IrOp};

impl Lowerer {
    pub(super) fn lower_byte_arithmetic(
        &mut self,
        op: Arithmetic,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let op = match op {
            Arithmetic::Sub => IrByteBinary::SubWrap,
            Arithmetic::Mul => IrByteBinary::MulWrap,
            Arithmetic::Div => IrByteBinary::Div,
        };
        self.lower_byte_binary(op, span)
    }

    pub(super) fn lower_bit_op(
        &mut self,
        shape: &tune_shape::Shape,
        byte_op: IrByteBinary,
        int_op: IntArithmetic,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if matches!(shape, tune_shape::Shape::Byte) {
            return self.lower_byte_binary(byte_op, span);
        }
        if matches!(shape, tune_shape::Shape::Size)
            && matches!(int_op, IntArithmetic::ShiftLeft | IntArithmetic::ShiftRight)
        {
            return self.lower_size_shift(int_op, span);
        }
        self.lower_int_arithmetic(int_op, span)
    }

    pub(super) fn lower_compare_byte(
        &mut self,
        op: IrIntComparison,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let op = match op {
            IrIntComparison::Equal => IrByteBinary::Equal,
            IrIntComparison::NotEqual => IrByteBinary::NotEqual,
            IrIntComparison::Less => IrByteBinary::Less,
            IrIntComparison::LessEqual => IrByteBinary::LessEqual,
            IrIntComparison::GreaterEqual => IrByteBinary::GreaterEqual,
        };
        self.lower_byte_binary(op, span)
    }

    pub(super) fn lower_byte_binary(
        &mut self,
        op: IrByteBinary,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::ByteBinary {
            dst,
            a: lhs,
            b: rhs,
            op,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }
}
