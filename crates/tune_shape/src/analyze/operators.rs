use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{BinaryOp, Expr};

use super::Analyzer;
use crate::{LiteralFact, Shape, expr_literal_fact, materialize::integer_value};

impl Analyzer<'_> {
    pub(super) fn analyze_binary(
        &mut self,
        op: BinaryOp,
        expr: &Expr,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Shape {
        let expected = self
            .expected_shape()
            .and_then(|expected| binary_operand_expected(op, expected))
            .cloned();
        let lhs_shape = if let Some(shape) = expected.as_ref() {
            self.analyze_expr_expected(lhs, shape)
        } else {
            self.analyze_expr(lhs)
        };
        if matches!(op, BinaryOp::And | BinaryOp::Or)
            && expected
                .as_ref()
                .is_none_or(|shape| matches!(shape, Shape::Bool))
            && Shape::Bool.accepts(&lhs_shape)
        {
            return self.analyze_short_circuit_binary(op, expr, lhs, rhs, lhs_shape);
        }
        let rhs_shape = if let Some(shape) = expected.as_ref() {
            self.analyze_expr_expected(rhs, shape)
        } else {
            self.analyze_expr(rhs)
        };
        match op {
            BinaryOp::Or | BinaryOp::And | BinaryOp::BitOr | BinaryOp::BitAnd
                if Shape::Bool.accepts(&lhs_shape) && Shape::Bool.accepts(&rhs_shape) =>
            {
                Shape::Bool
            }
            BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
                if Shape::Int.accepts(&lhs_shape) && Shape::Int.accepts(&rhs_shape) =>
            {
                Shape::Int
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
                if Shape::Int.accepts(&lhs_shape) && Shape::Int.accepts(&rhs_shape) =>
            {
                self.check_compile_time_int_fault(op, lhs, rhs, expr.span);
                Shape::Int
            }
            BinaryOp::ShiftLeft | BinaryOp::ShiftRight
                if Shape::Int.accepts(&lhs_shape) && Shape::Int.accepts(&rhs_shape) =>
            {
                self.check_compile_time_int_fault(op, lhs, rhs, expr.span);
                Shape::Int
            }
            BinaryOp::Add
                if Shape::Float.accepts(&lhs_shape) && Shape::Float.accepts(&rhs_shape) =>
            {
                Shape::Float
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div
                if Shape::Float.accepts(&lhs_shape) && Shape::Float.accepts(&rhs_shape) =>
            {
                Shape::Float
            }
            BinaryOp::Add if Shape::Size.accepts(&lhs_shape) && Shape::Size.accepts(&rhs_shape) => {
                self.check_compile_time_size_fault(op, lhs, rhs, expr.span);
                Shape::Size
            }
            BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
                if Shape::Size.accepts(&lhs_shape) && Shape::Size.accepts(&rhs_shape) =>
            {
                self.check_compile_time_size_fault(op, lhs, rhs, expr.span);
                Shape::Size
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
                if Shape::Byte.accepts(&lhs_shape) && Shape::Byte.accepts(&rhs_shape) =>
            {
                Shape::Byte
            }
            BinaryOp::RangeExclusive | BinaryOp::RangeInclusive
                if Shape::Int.accepts(&lhs_shape) && Shape::Int.accepts(&rhs_shape) =>
            {
                Shape::Range(Box::new(Shape::Int))
            }
            BinaryOp::RangeExclusive | BinaryOp::RangeInclusive
                if Shape::Size.accepts(&lhs_shape) && Shape::Size.accepts(&rhs_shape) =>
            {
                Shape::Range(Box::new(Shape::Size))
            }
            BinaryOp::Equal | BinaryOp::NotEqual
                if optional_none_equality_operand(&lhs_shape)
                    || optional_none_equality_operand(&rhs_shape) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Int.accepts(&lhs_shape) && Shape::Int.accepts(&rhs_shape) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Float.accepts(&lhs_shape) && Shape::Float.accepts(&rhs_shape) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Size.accepts(&lhs_shape) && Shape::Size.accepts(&rhs_shape) =>
            {
                Shape::Bool
            }
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
                if Shape::Byte.accepts(&lhs_shape) && Shape::Byte.accepts(&rhs_shape) =>
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
                    &lhs_shape,
                    &rhs_shape,
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
                if can_diagnose_operands(&lhs_shape, &rhs_shape) {
                    self.diagnostics.push(operator_mismatch(
                        op,
                        expected_operands(op),
                        &lhs_shape,
                        &rhs_shape,
                        expr.span,
                    ));
                }
                Shape::Hole
            }
        }
    }

    fn check_compile_time_int_fault(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        span: Option<Span>,
    ) {
        let (Some(lhs), Some(rhs)) = (int_literal(lhs), int_literal(rhs)) else {
            return;
        };
        let ok = match op {
            BinaryOp::Add => lhs.checked_add(rhs).is_some(),
            BinaryOp::Sub => lhs.checked_sub(rhs).is_some(),
            BinaryOp::Mul => lhs.checked_mul(rhs).is_some(),
            BinaryOp::Div => rhs != 0 && lhs.checked_div(rhs).is_some(),
            BinaryOp::Rem => rhs != 0 && lhs.checked_rem(rhs).is_some(),
            BinaryOp::ShiftLeft => {
                valid_int_shift(rhs) && lhs.checked_shl(u32::try_from(rhs).unwrap_or(0)).is_some()
            }
            BinaryOp::ShiftRight => {
                valid_int_shift(rhs) && lhs.checked_shr(u32::try_from(rhs).unwrap_or(0)).is_some()
            }
            _ => true,
        };
        if !ok {
            self.diagnostics.push(numeric_fault(op, span));
        }
    }

    fn check_compile_time_size_fault(
        &mut self,
        op: BinaryOp,
        lhs: &Expr,
        rhs: &Expr,
        span: Option<Span>,
    ) {
        let (Some(lhs), Some(rhs)) = (size_literal(lhs), size_literal(rhs)) else {
            return;
        };
        let ok = match op {
            BinaryOp::Add => lhs.checked_add(rhs).is_some(),
            BinaryOp::Sub => lhs.checked_sub(rhs).is_some(),
            BinaryOp::Mul => lhs.checked_mul(rhs).is_some(),
            BinaryOp::Div => rhs != 0 && lhs.checked_div(rhs).is_some(),
            BinaryOp::Rem => rhs != 0 && lhs.checked_rem(rhs).is_some(),
            BinaryOp::ShiftLeft => valid_size_shift(rhs) && lhs.checked_shl(rhs as u32).is_some(),
            BinaryOp::ShiftRight => valid_size_shift(rhs) && lhs.checked_shr(rhs as u32).is_some(),
            _ => true,
        };
        if !ok {
            self.diagnostics.push(numeric_fault(op, span));
        }
    }

    fn analyze_short_circuit_binary(
        &mut self,
        op: BinaryOp,
        expr: &Expr,
        lhs: &Expr,
        rhs: &Expr,
        lhs_shape: Shape,
    ) -> Shape {
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
    ) {
        return None;
    }
    match (op, expected) {
        (
            BinaryOp::Add
            | BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight,
            Shape::Byte,
        ) => Some(expected),
        (_, Shape::Byte) => None,
        (BinaryOp::Or | BinaryOp::And | BinaryOp::BitOr | BinaryOp::BitAnd, Shape::Bool) => {
            Some(expected)
        }
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

fn int_literal(expr: &Expr) -> Option<i64> {
    let LiteralFact::Numeric { text } = expr_literal_fact(expr)? else {
        return None;
    };
    integer_value(&text).and_then(|value| i64::try_from(value).ok())
}

fn size_literal(expr: &Expr) -> Option<u64> {
    let LiteralFact::Numeric { text } = expr_literal_fact(expr)? else {
        return None;
    };
    integer_value(&text).and_then(|value| u64::try_from(value).ok())
}

fn valid_int_shift(value: i64) -> bool {
    u32::try_from(value)
        .ok()
        .is_some_and(|shift| shift < i64::BITS)
}

fn valid_size_shift(value: u64) -> bool {
    u32::try_from(value)
        .ok()
        .is_some_and(|shift| shift < u64::BITS)
}

fn numeric_fault(op: BinaryOp, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::NUMERIC_OVERFLOW,
        "compile-time numeric operation cannot produce a valid value",
        span.unwrap_or_else(Span::synthetic),
        format!(
            "operator `{}` is proven to overflow or divide by zero",
            op_name(op)
        ),
    )
    .build()
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
        BinaryOp::Or | BinaryOp::And => "`Bool`, `Int`, or `Byte` operands",
        BinaryOp::Add => "compatible numeric operands",
        BinaryOp::RangeExclusive | BinaryOp::RangeInclusive => {
            "`Int`/`Int` or `Size`/`Size` endpoints"
        }
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual => {
            "`Int`/`Int`, `Float`/`Float`, `Size`/`Size`, or `Byte`/`Byte` operands"
        }
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
            "`Int`/`Int`, `Size`/`Size`, or `Byte`/`Byte` operands"
        }
        BinaryOp::BitOr | BinaryOp::BitAnd => "`Bool`, `Int`, or `Byte` operands",
        BinaryOp::BitXor => "`Int`/`Int` or `Byte`/`Byte` operands",
        BinaryOp::ShiftLeft | BinaryOp::ShiftRight => {
            "`Int`/`Int`, `Size`/`Size`, or `Byte`/`Byte` operands"
        }
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
