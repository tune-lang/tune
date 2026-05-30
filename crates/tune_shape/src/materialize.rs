use tune_diagnostics::Span;
use tune_hir::ExprId;

use crate::{LiteralFact, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commitment {
    PerUse,
    CommitBinding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializationPlan {
    pub target: Shape,
    pub commitment: Commitment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprMaterialization {
    pub expr: ExprId,
    pub plan: MaterializationPlan,
    pub span: Option<Span>,
}

#[must_use]
pub fn can_materialize(lit: &LiteralFact, target: &Shape) -> bool {
    match (lit, target) {
        (_, Shape::Hole) => true,
        (_, Shape::Never) => false,
        (LiteralFact::Numeric { text }, Shape::Byte) => {
            integer_value(text).is_some_and(|n| n <= 255)
        }
        (LiteralFact::Numeric { text }, Shape::Int) => {
            integer_value(text).is_some_and(|n| n <= i64::MAX as u128)
        }
        (LiteralFact::Numeric { text }, Shape::Size) => {
            integer_value(text).is_some_and(|n| n <= u64::MAX as u128)
        }
        (LiteralFact::Numeric { text }, Shape::Float) => float_value(text).is_some(),
        (LiteralFact::String { .. }, Shape::String) => true,
        (LiteralFact::Sequence { elements }, Shape::Sequence(element_shape)) => elements
            .iter()
            .all(|element| can_materialize(element, element_shape)),
        (LiteralFact::Bool(_), Shape::Bool) => true,
        (LiteralFact::None, Shape::Optional(_)) => true,
        (LiteralFact::Unit, Shape::Unit) => true,
        _ => false,
    }
}

pub(crate) fn integer_value(text: &str) -> Option<u128> {
    let normalized = text.replace('_', "");
    let (digits, radix) = integer_digits_and_radix(&normalized);
    if digits.is_empty() {
        return None;
    }
    u128::from_str_radix(digits, radix).ok()
}

fn float_value(text: &str) -> Option<f64> {
    let normalized = text.replace('_', "");
    normalized
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn integer_digits_and_radix(text: &str) -> (&str, u32) {
    if let Some(digits) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
        return (digits, 2);
    }
    if let Some(digits) = text.strip_prefix("0o").or_else(|| text.strip_prefix("0O")) {
        return (digits, 8);
    }
    if let Some(digits) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        return (digits, 16);
    }
    (text, 10)
}
