use tune_hir::shape::{ShapeExpr, ShapeExprKind};

use crate::{Shape, ShapeId, ShapeOrigin, ShapeStore};

pub fn intern_hir_shape(store: &mut ShapeStore, expr: &ShapeExpr) -> Option<ShapeId> {
    let shape = lower_hir_shape(expr);
    let origin = expr
        .span
        .map(ShapeOrigin::Annotation)
        .unwrap_or(ShapeOrigin::Synthetic);
    store.intern(shape, origin)
}

#[must_use]
pub fn lower_hir_shape(expr: &ShapeExpr) -> Shape {
    match &expr.kind {
        ShapeExprKind::Missing => Shape::Hole,
        ShapeExprKind::Named(name) => named_shape(name),
        ShapeExprKind::Sequence(element) => Shape::Sequence(Box::new(lower_hir_shape(element))),
        ShapeExprKind::Tuple(items) => Shape::Tuple(items.iter().map(lower_hir_shape).collect()),
        ShapeExprKind::Optional(inner) => Shape::Optional(Box::new(lower_hir_shape(inner))),
        ShapeExprKind::Union(items) => Shape::Union(items.iter().map(lower_hir_shape).collect()),
        ShapeExprKind::Callable { params, ret } => Shape::Callable {
            params: params.iter().map(lower_hir_shape).collect(),
            ret: Box::new(lower_hir_shape(ret)),
        },
    }
}

fn named_shape(name: &str) -> Shape {
    match name {
        "Never" => Shape::Never,
        "()" | "Unit" => Shape::Unit,
        "Int" => Shape::Int,
        "Float" => Shape::Float,
        "Size" => Shape::Size,
        "Byte" => Shape::Byte,
        "Bool" => Shape::Bool,
        "String" => Shape::String,
        _ => Shape::Struct(name.to_owned()),
    }
}
