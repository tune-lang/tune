use tune_hir::expr::{Expr, ExprKind, LiteralKind};
use tune_hir::module::Module;
use tune_resolve::{NameTarget, PreludeVariant, ResolvedModule, VariantId};

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
pub fn expr_shape_fact(expr: &Expr, module: &Module, resolved: &ResolvedModule) -> Option<Shape> {
    match &expr.kind {
        ExprKind::Name(_) => {
            let target = name_target(expr, resolved)?;
            variant_shape(target, None, module)
        }
        ExprKind::Call { callee, args } => {
            variant_constructor_shape(callee, args, module, resolved)
        }
        ExprKind::Propagate(inner) => result_ok_shape(expr_shape_fact(inner, module, resolved)?),
        _ => None,
    }
}

fn variant_constructor_shape(
    callee: &Expr,
    args: &[Expr],
    module: &Module,
    resolved: &ResolvedModule,
) -> Option<Shape> {
    let arg_shape = args
        .first()
        .and_then(|arg| expr_shape_fact(arg, module, resolved))
        .unwrap_or(Shape::Hole);

    variant_shape(name_target(callee, resolved)?, Some(arg_shape), module)
}

fn name_target(expr: &Expr, resolved: &ResolvedModule) -> Option<NameTarget> {
    resolved
        .name_refs
        .iter()
        .find(|name_ref| name_ref.expr == expr.id)
        .map(|name_ref| name_ref.target)
}

fn variant_shape(target: NameTarget, arg_shape: Option<Shape>, module: &Module) -> Option<Shape> {
    match target {
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Ok)) => {
            Some(Shape::Result {
                ok: Box::new(arg_shape.unwrap_or(Shape::Hole)),
                err: Box::new(Shape::Hole),
            })
        }
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Error)) => {
            Some(Shape::Result {
                ok: Box::new(Shape::Hole),
                err: Box::new(arg_shape.unwrap_or(Shape::Hole)),
            })
        }
        NameTarget::Variant(VariantId::Member(variant)) => module
            .items
            .iter()
            .find(|item| item.id == variant.owner)
            .and_then(|item| item.name.as_ref())
            .map(|name| Shape::Enum(name.clone())),
        _ => None,
    }
}

fn result_ok_shape(shape: Shape) -> Option<Shape> {
    match shape {
        Shape::Result { ok, .. } => Some(*ok),
        _ => None,
    }
}
