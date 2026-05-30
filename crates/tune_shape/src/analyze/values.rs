use tune_hir::expr::{Expr, ExprKind, UnaryOp};

use super::Analyzer;
use crate::{
    Commitment, ExprMaterialization, MaterializationPlan, Shape, can_materialize, expr_literal_fact,
};

impl Analyzer<'_> {
    pub(super) fn literal_or_sequence_shape(&mut self, expr: &Expr) -> Shape {
        if let ExprKind::Sequence(elements) = &expr.kind {
            for element in elements {
                self.analyze_expr(element);
            }
        }
        if let ExprKind::Tuple(items) = &expr.kind {
            return Shape::product(items.iter().map(|item| self.analyze_expr(item)).collect());
        }
        let Some(literal) = expr_literal_fact(expr) else {
            return Shape::Hole;
        };
        if literal.is_numeric()
            && let Some(expected) = self.expected_shape()
            && can_materialize(&literal, expected)
        {
            let target = expected.clone();
            self.materializations.push(ExprMaterialization {
                expr: expr.id,
                plan: MaterializationPlan {
                    target: target.clone(),
                    commitment: Commitment::PerUse,
                },
                span: expr.span,
            });
            return target;
        }
        Shape::Literal(literal)
    }

    pub(super) fn analyze_unary(&mut self, op: UnaryOp, expr: &Expr) -> Shape {
        let shape = self.analyze_expr(expr);
        match op {
            UnaryOp::Invert if Shape::Bool.accepts(&shape) => Shape::Bool,
            UnaryOp::Neg | UnaryOp::Invert if Shape::Int.accepts(&shape) => Shape::Int,
            UnaryOp::Invert if Shape::Byte.accepts(&shape) => Shape::Byte,
            UnaryOp::Neg | UnaryOp::Invert => Shape::Hole,
        }
    }
}
