use tune_diagnostics::Span;
use tune_hir::expr::BinaryOp;

use crate::lower::{IrLowerError, Lowerer};
use crate::{IrIntComparison, IrOp};

impl Lowerer {
    pub(super) fn lower_binary(
        &mut self,
        op: BinaryOp,
        shape: &tune_shape::Shape,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        match op {
            BinaryOp::Add => self.lower_add(shape, span),
            BinaryOp::RangeExclusive => self.lower_range_int(false, span),
            BinaryOp::RangeInclusive => self.lower_range_int(true, span),
            BinaryOp::Greater => self.lower_greater_int(span),
            BinaryOp::Is | BinaryOp::Equal => self.lower_compare_int(IrIntComparison::Equal, span),
            BinaryOp::IsNot | BinaryOp::NotEqual => {
                self.lower_compare_int(IrIntComparison::NotEqual, span)
            }
            BinaryOp::Less => self.lower_compare_int(IrIntComparison::Less, span),
            BinaryOp::LessEqual => self.lower_compare_int(IrIntComparison::LessEqual, span),
            BinaryOp::GreaterEqual => self.lower_compare_int(IrIntComparison::GreaterEqual, span),
            _ => Err(IrLowerError::UnsupportedOp("binary op")),
        }
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

    fn lower_add_float(&mut self, _span: Option<Span>) -> Result<(), IrLowerError> {
        let rhs = self.pop("binary rhs")?;
        let lhs = self.pop("binary lhs")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::AddFloat {
            dst,
            a: lhs,
            b: rhs,
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
}
