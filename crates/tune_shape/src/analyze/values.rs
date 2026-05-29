use tune_hir::expr::{Expr, ExprKind, UnaryOp};

use super::Analyzer;
use crate::{Shape, expr_literal_fact};

impl Analyzer<'_> {
    pub(super) fn literal_or_sequence_shape(&mut self, expr: &Expr) -> Shape {
        if let ExprKind::Sequence(elements) = &expr.kind {
            for element in elements {
                self.analyze_expr(element);
            }
        }
        if let ExprKind::Tuple(items) = &expr.kind {
            return Shape::Tuple(items.iter().map(|item| self.analyze_expr(item)).collect());
        }
        let Some(literal) = expr_literal_fact(expr) else {
            return Shape::Hole;
        };
        if literal.is_numeric()
            && let Some(expected) = self.expected_shape()
            && expected.accepts(&Shape::Literal(literal.clone()))
        {
            return expected.clone();
        }
        Shape::Literal(literal)
    }

    pub(super) fn analyze_unary(&mut self, op: UnaryOp, expr: &Expr) -> Shape {
        let shape = self.analyze_expr(expr);
        match op {
            UnaryOp::Not => Shape::Bool,
            UnaryOp::Neg | UnaryOp::BitNot if Shape::Int.accepts(&shape) => Shape::Int,
            UnaryOp::BitNot if Shape::Byte.accepts(&shape) => Shape::Byte,
            UnaryOp::Neg | UnaryOp::BitNot => Shape::Hole,
        }
    }
}
