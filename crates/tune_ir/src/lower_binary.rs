use tune_diagnostics::Span;
use tune_hir::expr::BinaryOp;

use crate::lower::{IrLowerError, Lowerer};
use crate::{IrIntComparison, IrOp};

impl Lowerer {
    pub(super) fn lower_binary(
        &mut self,
        op: BinaryOp,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        match op {
            BinaryOp::Add => self.lower_add_int(span),
            BinaryOp::RangeExclusive => self.lower_range_int(false, span),
            BinaryOp::RangeInclusive => self.lower_range_int(true, span),
            BinaryOp::Greater => self.lower_greater_int(span),
            BinaryOp::Equal => self.lower_compare_int(IrIntComparison::Equal, span),
            BinaryOp::NotEqual => self.lower_compare_int(IrIntComparison::NotEqual, span),
            BinaryOp::Less => self.lower_compare_int(IrIntComparison::Less, span),
            BinaryOp::LessEqual => self.lower_compare_int(IrIntComparison::LessEqual, span),
            BinaryOp::GreaterEqual => self.lower_compare_int(IrIntComparison::GreaterEqual, span),
            _ => Err(IrLowerError::UnsupportedOp("binary op")),
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
