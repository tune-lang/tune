use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{BinaryOp, Expr};

use super::Analyzer;
use crate::Shape;

impl Analyzer<'_> {
    pub(super) fn analyze_binary(
        &mut self,
        op: BinaryOp,
        expr: &Expr,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Shape {
        let lhs = self.analyze_expr(lhs);
        let rhs = self.analyze_expr(rhs);
        match op {
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
                if Shape::Bool.accepts(&lhs) && Shape::Bool.accepts(&rhs) =>
            {
                Shape::Bool
            }
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Int
            }
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Int
            }
            BinaryOp::RangeExclusive | BinaryOp::RangeInclusive
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Range(Box::new(Shape::Int))
            }
            BinaryOp::RangeExclusive | BinaryOp::RangeInclusive
                if Shape::Size.accepts(&lhs) && Shape::Size.accepts(&rhs) =>
            {
                Shape::Range(Box::new(Shape::Size))
            }
            BinaryOp::Is
            | BinaryOp::IsNot
            | BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Bool
            }
            BinaryOp::Is
            | BinaryOp::IsNot
            | BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                self.diagnostics
                    .push(operator_mismatch(op, &lhs, &rhs, expr.span));
                Shape::Bool
            }
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
            | BinaryOp::RangeExclusive
            | BinaryOp::RangeInclusive => Shape::Hole,
        }
    }
}

fn operator_mismatch(op: BinaryOp, lhs: &Shape, rhs: &Shape, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::SHAPE_MISMATCH,
        "operator operands do not match executable integer operation",
        span.unwrap_or_else(Span::synthetic),
        format!(
            "operator `{}` expected `Int` operands, got `{lhs:?}` and `{rhs:?}`",
            op_name(op)
        ),
    )
    .build()
}

fn op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Is => "is",
        BinaryOp::IsNot => "is not",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "~=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        _ => "<operator>",
    }
}
