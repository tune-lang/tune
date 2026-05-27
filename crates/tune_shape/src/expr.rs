use tune_hir::expr::{Expr, ExprKind, LiteralKind};
use tune_resolve::{NameTarget, PreludeVariant, ResolvedModule};

use crate::{LiteralFact, Shape};

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

#[must_use]
pub fn expr_shape_fact(expr: &Expr, resolved: &ResolvedModule) -> Option<Shape> {
    match &expr.kind {
        ExprKind::Call { callee, args } => variant_constructor_shape(callee, args, resolved),
        ExprKind::Propagate(inner) => result_ok_shape(expr_shape_fact(inner, resolved)?),
        _ => None,
    }
}

fn variant_constructor_shape(
    callee: &Expr,
    args: &[Expr],
    resolved: &ResolvedModule,
) -> Option<Shape> {
    let target = resolved
        .name_refs
        .iter()
        .find(|name_ref| name_ref.expr == callee.id)
        .map(|name_ref| name_ref.target)?;

    let arg_shape = args
        .first()
        .and_then(|arg| expr_shape_fact(arg, resolved))
        .unwrap_or(Shape::Hole);

    match target {
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Ok)) => {
            Some(Shape::Result {
                ok: Box::new(arg_shape),
                err: Box::new(Shape::Hole),
            })
        }
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Error)) => {
            Some(Shape::Result {
                ok: Box::new(Shape::Hole),
                err: Box::new(arg_shape),
            })
        }
        _ => None,
    }
}

fn result_ok_shape(shape: Shape) -> Option<Shape> {
    match shape {
        Shape::Result { ok, .. } => Some(*ok),
        _ => None,
    }
}
