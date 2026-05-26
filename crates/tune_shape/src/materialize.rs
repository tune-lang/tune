use crate::{LiteralFact, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Commitment {
    PerUse,
    CommitBinding,
}

#[derive(Debug, Clone)]
pub struct MaterializationPlan {
    pub target: Shape,
    pub commitment: Commitment,
}

#[must_use]
pub fn can_materialize(lit: &LiteralFact, target: &Shape) -> bool {
    match (lit, target) {
        (_, Shape::Hole) => true,
        (_, Shape::Never) => false,
        (LiteralFact::Numeric { text }, Shape::Byte) => {
            numeric_value(text).is_some_and(|n| n <= 255)
        }
        (LiteralFact::Numeric { text }, Shape::Int | Shape::Size) => numeric_value(text).is_some(),
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

fn numeric_value(text: &str) -> Option<u128> {
    let mut value = 0u128;
    let mut saw_digit = false;

    for byte in text.bytes() {
        match byte {
            b'_' => {}
            b'0'..=b'9' => {
                saw_digit = true;
                let digit = u128::from(byte - b'0');
                value = value.checked_mul(10)?.checked_add(digit)?;
            }
            _ => return None,
        }
    }

    saw_digit.then_some(value)
}

fn float_value(text: &str) -> Option<f64> {
    let normalized = text.replace('_', "");
    normalized
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}
