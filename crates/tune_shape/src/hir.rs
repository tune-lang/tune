use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::shape::{ShapeExpr, ShapeExprKind};
use tune_resolve::{BindingKind, Scope};

use crate::{Shape, ShapeId, ShapeOrigin, ShapeStore};

#[derive(Debug, Clone)]
pub struct LoweredShape {
    pub shape: Shape,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn intern_hir_shape(store: &mut ShapeStore, expr: &ShapeExpr) -> Option<ShapeId> {
    let shape = lower_hir_shape(expr);
    let origin = expr
        .span
        .map(ShapeOrigin::Annotation)
        .unwrap_or(ShapeOrigin::Synthetic);
    store.intern(shape, origin)
}

pub fn intern_resolved_hir_shape(
    store: &mut ShapeStore,
    expr: &ShapeExpr,
    scope: &Scope,
) -> (Option<ShapeId>, Vec<Diagnostic>) {
    let lowered = lower_resolved_hir_shape(expr, scope);
    let origin = expr
        .span
        .map(ShapeOrigin::Annotation)
        .unwrap_or(ShapeOrigin::Synthetic);
    let id = store.intern(lowered.shape, origin);
    (id, lowered.diagnostics)
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

#[must_use]
pub fn lower_resolved_hir_shape(expr: &ShapeExpr, scope: &Scope) -> LoweredShape {
    match &expr.kind {
        ShapeExprKind::Missing => LoweredShape {
            shape: Shape::Hole,
            diagnostics: Vec::new(),
        },
        ShapeExprKind::Named(name) => lower_named_shape(name, expr.span, scope),
        ShapeExprKind::Sequence(element) => {
            let lowered = lower_resolved_hir_shape(element, scope);
            LoweredShape {
                shape: Shape::Sequence(Box::new(lowered.shape)),
                diagnostics: lowered.diagnostics,
            }
        }
        ShapeExprKind::Tuple(items) => lower_many(items, scope, Shape::Tuple),
        ShapeExprKind::Optional(inner) => {
            let lowered = lower_resolved_hir_shape(inner, scope);
            LoweredShape {
                shape: Shape::Optional(Box::new(lowered.shape)),
                diagnostics: lowered.diagnostics,
            }
        }
        ShapeExprKind::Union(items) => lower_many(items, scope, Shape::Union),
        ShapeExprKind::Callable { params, ret } => {
            let mut lowered_params = Vec::new();
            let mut diagnostics = Vec::new();
            for param in params {
                let lowered = lower_resolved_hir_shape(param, scope);
                lowered_params.push(lowered.shape);
                diagnostics.extend(lowered.diagnostics);
            }

            let lowered_ret = lower_resolved_hir_shape(ret, scope);
            diagnostics.extend(lowered_ret.diagnostics);
            LoweredShape {
                shape: Shape::Callable {
                    params: lowered_params,
                    ret: Box::new(lowered_ret.shape),
                },
                diagnostics,
            }
        }
    }
}

fn lower_many(
    items: &[ShapeExpr],
    scope: &Scope,
    wrap: impl FnOnce(Vec<Shape>) -> Shape,
) -> LoweredShape {
    let mut shapes = Vec::new();
    let mut diagnostics = Vec::new();
    for item in items {
        let lowered = lower_resolved_hir_shape(item, scope);
        shapes.push(lowered.shape);
        diagnostics.extend(lowered.diagnostics);
    }

    LoweredShape {
        shape: wrap(shapes),
        diagnostics,
    }
}

fn lower_named_shape(name: &str, span: Option<Span>, scope: &Scope) -> LoweredShape {
    if let Some(shape) = builtin_shape(name) {
        return LoweredShape {
            shape,
            diagnostics: Vec::new(),
        };
    }

    match scope.get(name).map(|binding| binding.kind) {
        Some(BindingKind::Struct) => LoweredShape {
            shape: Shape::Struct(name.to_owned()),
            diagnostics: Vec::new(),
        },
        Some(BindingKind::Enum) => LoweredShape {
            shape: Shape::Enum(name.to_owned()),
            diagnostics: Vec::new(),
        },
        _ => {
            let span = span.unwrap_or_else(Span::synthetic);
            LoweredShape {
                shape: Shape::Hole,
                diagnostics: vec![
                    Diagnostic::error(
                        codes::UNRESOLVED_NAME,
                        format!("unknown shape `{name}`"),
                        span,
                        "this shape name is not in scope",
                    )
                    .build(),
                ],
            }
        }
    }
}

fn named_shape(name: &str) -> Shape {
    if let Some(shape) = builtin_shape(name) {
        return shape;
    }

    Shape::Struct(name.to_owned())
}

fn builtin_shape(name: &str) -> Option<Shape> {
    match name {
        "Never" => Some(Shape::Never),
        "()" | "Unit" => Some(Shape::Unit),
        "Int" => Some(Shape::Int),
        "Float" => Some(Shape::Float),
        "Size" => Some(Shape::Size),
        "Byte" => Some(Shape::Byte),
        "Bool" => Some(Shape::Bool),
        "String" => Some(Shape::String),
        _ => None,
    }
}
