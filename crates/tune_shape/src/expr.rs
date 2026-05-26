use tune_hir::expr::{Expr, ExprKind, LiteralKind};

use crate::LiteralFact;

#[must_use]
pub fn expr_literal_fact(expr: &Expr) -> Option<LiteralFact> {
    match &expr.kind {
        ExprKind::Literal(literal) => literal_fact(literal),
        ExprKind::Sequence(elements) => elements
            .iter()
            .map(expr_literal_fact)
            .collect::<Option<Vec<_>>>()
            .map(|elements| LiteralFact::Sequence { elements }),
        _ => None,
    }
}

fn literal_fact(literal: &LiteralKind) -> Option<LiteralFact> {
    match literal {
        LiteralKind::Int(text) | LiteralKind::Float(text) => {
            Some(LiteralFact::Numeric { text: text.clone() })
        }
        LiteralKind::String(text) => Some(LiteralFact::String {
            segments: vec![text.clone()],
        }),
        LiteralKind::Bool(value) => Some(LiteralFact::Bool(*value)),
        LiteralKind::None => Some(LiteralFact::None),
    }
}
