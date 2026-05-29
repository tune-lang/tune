use tune_diagnostics::Span;
use tune_hir::expr::BinaryOp;

use crate::lower::{IrLowerError, Lowerer};
use crate::{IrByteBinary, IrIntComparison, IrOp};

impl Lowerer {
    pub(super) fn lower_binary(
        &mut self,
        op: BinaryOp,
        shape: &tune_shape::Shape,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        match op {
            BinaryOp::Add => self.lower_add(shape, span),
            BinaryOp::Sub => self.lower_arithmetic(shape, Arithmetic::Sub, span),
            BinaryOp::Mul => self.lower_arithmetic(shape, Arithmetic::Mul, span),
            BinaryOp::Div => self.lower_arithmetic(shape, Arithmetic::Div, span),
            BinaryOp::Rem => self.lower_remainder(shape, span),
            BinaryOp::BitAnd => {
                self.lower_bit_op(shape, IrByteBinary::BitAnd, IntArithmetic::BitAnd, span)
            }
            BinaryOp::BitOr => {
                self.lower_bit_op(shape, IrByteBinary::BitOr, IntArithmetic::BitOr, span)
            }
            BinaryOp::BitXor => {
                self.lower_bit_op(shape, IrByteBinary::BitXor, IntArithmetic::BitXor, span)
            }
            BinaryOp::ShiftLeft => self.lower_bit_op(
                shape,
                IrByteBinary::ShiftLeft,
                IntArithmetic::ShiftLeft,
                span,
            ),
            BinaryOp::ShiftRight => self.lower_bit_op(
                shape,
                IrByteBinary::ShiftRight,
                IntArithmetic::ShiftRight,
                span,
            ),
            BinaryOp::RangeExclusive => self.lower_range_int(false, span),
            BinaryOp::RangeInclusive => self.lower_range_int(true, span),
            BinaryOp::Greater => self.lower_greater(shape, span),
            BinaryOp::Equal => self.lower_compare(shape, IrIntComparison::Equal, span),
            BinaryOp::NotEqual => self.lower_compare(shape, IrIntComparison::NotEqual, span),
            BinaryOp::Less => self.lower_compare(shape, IrIntComparison::Less, span),
            BinaryOp::LessEqual => self.lower_compare(shape, IrIntComparison::LessEqual, span),
            BinaryOp::GreaterEqual => {
                self.lower_compare(shape, IrIntComparison::GreaterEqual, span)
            }
            _ => Err(IrLowerError::UnsupportedOp("binary op")),
        }
    }

    fn lower_int_arithmetic(
        &mut self,
        op: IntArithmetic,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        match op {
            IntArithmetic::Sub => self.push_op(IrOp::SubInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::Mul => self.push_op(IrOp::MulInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::Div => self.push_op(IrOp::DivInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::Rem => self.push_op(IrOp::RemInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::BitAnd => self.push_op(IrOp::BitAndInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::BitOr => self.push_op(IrOp::BitOrInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::BitXor => self.push_op(IrOp::BitXorInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::ShiftLeft => self.push_op(IrOp::ShiftLeftInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            IntArithmetic::ShiftRight => self.push_op(IrOp::ShiftRightInt {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
        }
        self.stack.push(dst);
        Ok(())
    }

    fn lower_arithmetic(
        &mut self,
        shape: &tune_shape::Shape,
        op: Arithmetic,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if matches!(shape, tune_shape::Shape::Float) {
            return self.lower_float_arithmetic(op, span);
        }
        if matches!(shape, tune_shape::Shape::Size) {
            return self.lower_size_arithmetic(op, span);
        }
        if matches!(shape, tune_shape::Shape::Byte) {
            return self.lower_byte_arithmetic(op, span);
        }
        self.lower_int_arithmetic(op.into(), span)
    }

    fn lower_remainder(
        &mut self,
        shape: &tune_shape::Shape,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if matches!(shape, tune_shape::Shape::Size) {
            return self.lower_size_rem(span);
        }
        if matches!(shape, tune_shape::Shape::Byte) {
            return self.lower_byte_binary(IrByteBinary::Rem, span);
        }
        self.lower_int_arithmetic(IntArithmetic::Rem, span)
    }

    fn lower_add(
        &mut self,
        shape: &tune_shape::Shape,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        match shape {
            tune_shape::Shape::Float => self.lower_add_float(span),
            tune_shape::Shape::Size => self.lower_add_size(span),
            tune_shape::Shape::Byte => self.lower_add_byte(span),
            _ => self.lower_add_int(span),
        }
    }

    fn lower_add_int(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::AddInt {
            dst,
            a: lhs,
            b: rhs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_greater(
        &mut self,
        shape: &tune_shape::Shape,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if matches!(shape, tune_shape::Shape::Float) {
            return self.lower_greater_float(span);
        }
        if matches!(shape, tune_shape::Shape::Size) {
            return self.lower_greater_size(span);
        }
        if matches!(shape, tune_shape::Shape::Byte) {
            return self.lower_byte_binary(IrByteBinary::Greater, span);
        }
        self.lower_greater_int(span)
    }

    fn lower_greater_int(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::GreaterInt {
            dst,
            a: lhs,
            b: rhs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_greater_float(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::GreaterFloat {
            dst,
            a: lhs,
            b: rhs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_greater_size(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::GreaterSize {
            dst,
            a: lhs,
            b: rhs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_add_float(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::AddFloat {
            dst,
            a: lhs,
            b: rhs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_float_arithmetic(
        &mut self,
        op: Arithmetic,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        match op {
            Arithmetic::Sub => self.push_op(IrOp::SubFloat {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            Arithmetic::Mul => self.push_op(IrOp::MulFloat {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            Arithmetic::Div => self.push_op(IrOp::DivFloat {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
        }
        self.stack.push(dst);
        Ok(())
    }

    fn lower_size_arithmetic(
        &mut self,
        op: Arithmetic,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        match op {
            Arithmetic::Sub => self.push_op(IrOp::SubSizeChecked {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            Arithmetic::Mul => self.push_op(IrOp::MulSizeChecked {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
            Arithmetic::Div => self.push_op(IrOp::DivSize {
                dst,
                a: lhs,
                b: rhs,
                span,
            }),
        }
        self.stack.push(dst);
        Ok(())
    }

    fn lower_size_rem(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::RemSize {
            dst,
            a: lhs,
            b: rhs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_byte_arithmetic(
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

    fn lower_bit_op(
        &mut self,
        shape: &tune_shape::Shape,
        byte_op: IrByteBinary,
        int_op: IntArithmetic,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if matches!(shape, tune_shape::Shape::Byte) {
            return self.lower_byte_binary(byte_op, span);
        }
        self.lower_int_arithmetic(int_op, span)
    }

    fn lower_byte_binary(
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

    fn lower_add_size(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::AddSizeChecked {
            dst,
            a: lhs,
            b: rhs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_add_byte(&mut self, _span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::AddByteWrap {
            dst,
            a: lhs,
            b: rhs,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_range_int(&mut self, inclusive: bool, span: Option<Span>) -> Result<(), IrLowerError> {
        let end = self.pop("range end")?;
        let start = self.pop("range start")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::RangeInt {
            dst,
            start,
            end,
            inclusive,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_compare_int(
        &mut self,
        op: IrIntComparison,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CompareInt {
            dst,
            a: lhs,
            b: rhs,
            op,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_compare(
        &mut self,
        shape: &tune_shape::Shape,
        op: IrIntComparison,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if matches!(shape, tune_shape::Shape::Float) {
            return self.lower_compare_float(op, span);
        }
        if matches!(shape, tune_shape::Shape::Size) {
            return self.lower_compare_size(op, span);
        }
        if matches!(shape, tune_shape::Shape::Byte) {
            let op = match op {
                IrIntComparison::Equal => IrByteBinary::Equal,
                IrIntComparison::NotEqual => IrByteBinary::NotEqual,
                IrIntComparison::Less => IrByteBinary::Less,
                IrIntComparison::LessEqual => IrByteBinary::LessEqual,
                IrIntComparison::GreaterEqual => IrByteBinary::GreaterEqual,
            };
            return self.lower_byte_binary(op, span);
        }
        self.lower_compare_int(op, span)
    }

    fn lower_compare_float(
        &mut self,
        op: IrIntComparison,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CompareFloat {
            dst,
            a: lhs,
            b: rhs,
            op,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_compare_size(
        &mut self,
        op: IrIntComparison,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CompareSize {
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

#[derive(Clone, Copy)]
enum Arithmetic {
    Sub,
    Mul,
    Div,
}

impl From<Arithmetic> for IntArithmetic {
    fn from(value: Arithmetic) -> Self {
        match value {
            Arithmetic::Sub => Self::Sub,
            Arithmetic::Mul => Self::Mul,
            Arithmetic::Div => Self::Div,
        }
    }
}

enum IntArithmetic {
    Sub,
    Mul,
    Div,
    Rem,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
}
