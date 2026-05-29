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
        if matches!(op, BinaryOp::And | BinaryOp::Or) {
            return self.analyze_short_circuit_binary(op, expr, lhs, rhs);
        }
        let expected = self
            .expected_shape()
            .and_then(|expected| binary_operand_expected(op, expected))
            .cloned();
        let lhs = if let Some(shape) = expected.as_ref() {
            self.analyze_expr_expected(lhs, shape)
        } else {
            self.analyze_expr(lhs)
        };
        let rhs = if let Some(shape) = expected.as_ref() {
            self.analyze_expr_expected(rhs, shape)
        } else {
            self.analyze_expr(rhs)
        };
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

    fn analyze_short_circuit_binary(
        &mut self,
        op: BinaryOp,
        expr: &Expr,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Shape {
        let lhs_shape = self.analyze_expr(lhs);
        let entry = self.frame.clone();
        match op {
            BinaryOp::And => self.apply_condition_narrowing(lhs, true),
            BinaryOp::Or => self.apply_condition_narrowing(lhs, false),
            _ => {}
        }
        let rhs_shape = self.analyze_expr(rhs);
        self.frame = entry;
        if Shape::Bool.accepts(&lhs_shape) && Shape::Bool.accepts(&rhs_shape) {
            Shape::Bool
        } else {
            self.diagnostics.push(operator_mismatch(
                op,
                expected_operands(op),
                &lhs_shape,
                &rhs_shape,
                expr.span,
            ));
            Shape::Bool
        }
    }
}

fn binary_operand_expected(op: BinaryOp, expected: &Shape) -> Option<&Shape> {
    if !matches!(
        op,
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
    ) {
        return None;
    }
    match (op, expected) {
        (BinaryOp::Add, Shape::Byte) => Some(expected),
        (_, Shape::Byte) => None,
        (_, Shape::Int | Shape::Float | Shape::Size) => Some(expected),
        _ => None,
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
        BinaryOp::Or | BinaryOp::And => "`Bool` operands",
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
