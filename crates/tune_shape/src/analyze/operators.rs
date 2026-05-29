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
            BinaryOp::Or | BinaryOp::And
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
            BinaryOp::Add if Shape::Float.accepts(&lhs) && Shape::Float.accepts(&rhs) => {
                Shape::Float
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                if Shape::Float.accepts(&lhs) && Shape::Float.accepts(&rhs) =>
            {
                Shape::Float
            }
            BinaryOp::Add if Shape::Size.accepts(&lhs) && Shape::Size.accepts(&rhs) => Shape::Size,
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                if Shape::Size.accepts(&lhs) && Shape::Size.accepts(&rhs) =>
            {
                Shape::Size
            }
            BinaryOp::Add if Shape::Byte.accepts(&lhs) && Shape::Byte.accepts(&rhs) => Shape::Byte,
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
            BinaryOp::Equal | BinaryOp::NotEqual
                if optional_none_equality_operand(&lhs) || optional_none_equality_operand(&rhs) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Int.accepts(&lhs) && Shape::Int.accepts(&rhs) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Float.accepts(&lhs) && Shape::Float.accepts(&rhs) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Size.accepts(&lhs) && Shape::Size.accepts(&rhs) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                self.diagnostics.push(operator_mismatch(
                    op,
                    expected_operands(op),
                    &lhs,
                    &rhs,
                    expr.span,
                ));
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
            | BinaryOp::RangeInclusive => {
                if can_diagnose_operands(&lhs, &rhs) {
                    self.diagnostics.push(operator_mismatch(
                        op,
                        expected_operands(op),
                        &lhs,
                        &rhs,
                        expr.span,
                    ));
                }
                Shape::Hole
            }
        }
    }
}

fn optional_none_equality_operand(shape: &Shape) -> bool {
    matches!(
        shape,
        Shape::Optional(_) | Shape::Literal(crate::LiteralFact::None)
    )
}

fn operator_mismatch(
    op: BinaryOp,
    expected: &'static str,
    lhs: &Shape,
    rhs: &Shape,
    span: Option<Span>,
) -> Diagnostic {
    Diagnostic::error(
        codes::SHAPE_MISMATCH,
        "operator operands do not match an executable operation",
        span.unwrap_or_else(Span::synthetic),
        format!(
            "operator `{}` expected {expected}, got `{lhs:?}` and `{rhs:?}`",
            op_name(op)
        ),
    )
    .build()
}

fn can_diagnose_operands(lhs: &Shape, rhs: &Shape) -> bool {
    !matches!(lhs, Shape::Hole | Shape::Never) && !matches!(rhs, Shape::Hole | Shape::Never)
}

fn expected_operands(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Or | BinaryOp::And => "`Bool`/`Bool` or `Int`/`Int` operands",
        BinaryOp::Add => "compatible numeric operands",
        BinaryOp::RangeExclusive | BinaryOp::RangeInclusive => {
            "`Int`/`Int` or `Size`/`Size` endpoints"
        }
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual => "`Int`/`Int`, `Float`/`Float`, or `Size`/`Size` operands",
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
            "`Int`/`Int` or `Size`/`Size` operands"
        }
        BinaryOp::BitOr
        | BinaryOp::BitXor
        | BinaryOp::BitAnd
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight => "`Int` operands",
    }
}

fn op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::BitAnd => "&",
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::RangeExclusive => "..",
        BinaryOp::RangeInclusive => "..=",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "~=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
    }
}
