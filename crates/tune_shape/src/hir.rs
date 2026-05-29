use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{BindingKind, Scope};

use crate::{
    MemberRequirement, NominalShape, Shape, ShapeId, ShapeOrigin, ShapeStore,
    builtin::{builtin_generic_shape, builtin_shape},
};

#[derive(Debug, Clone)]
pub struct LoweredShape {
    pub shape: Shape,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn alloc_hir_shape(store: &mut ShapeStore, expr: &ShapeExpr) -> Option<ShapeId> {
    let shape = lower_hir_shape(expr);
    let origin = expr
        .span
        .map(ShapeOrigin::Annotation)
        .unwrap_or(ShapeOrigin::Synthetic);
    store.alloc(shape, origin)
}

pub fn alloc_resolved_hir_shape(
    store: &mut ShapeStore,
    expr: &ShapeExpr,
    scope: &Scope,
) -> (Option<ShapeId>, Vec<Diagnostic>) {
    let lowered = lower_resolved_hir_shape(expr, scope);
    let origin = expr
        .span
        .map(ShapeOrigin::Annotation)
        .unwrap_or(ShapeOrigin::Synthetic);
    let id = store.alloc(lowered.shape, origin);
    (id, lowered.diagnostics)
}

#[must_use]
pub fn lower_hir_shape(expr: &ShapeExpr) -> Shape {
    match &expr.kind {
        ShapeExprKind::Missing => Shape::Hole,
        ShapeExprKind::Named(name) => named_shape(name),
        ShapeExprKind::Generic { name, args } => {
            generic_shape(name, args.iter().map(lower_hir_shape).collect())
        }
        ShapeExprKind::Sequence(element) => Shape::Sequence(Box::new(lower_hir_shape(element))),
        ShapeExprKind::Tuple(items) => Shape::product(items.iter().map(lower_hir_shape).collect()),
        ShapeExprKind::Optional(inner) => Shape::Optional(Box::new(lower_hir_shape(inner))),
        ShapeExprKind::Union(items) => Shape::Union(items.iter().map(lower_hir_shape).collect()),
        ShapeExprKind::Structural(requirements) => Shape::Structural(
            requirements
                .iter()
                .map(|requirement| match &requirement.kind {
                    StructuralShapeRequirementKind::Field { shape } => MemberRequirement::Field {
                        name: requirement.name.clone(),
                        shape: shape.as_ref().map(lower_hir_shape),
                    },
                    StructuralShapeRequirementKind::Callable { params, ret } => {
                        MemberRequirement::Callable {
                            name: requirement.name.clone(),
                            params: params.iter().map(lower_hir_shape).collect(),
                            ret: ret.as_ref().map(lower_hir_shape),
                        }
                    }
                })
                .collect(),
        ),
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
        ShapeExprKind::Generic { name, args } => lower_generic_shape(name, args, expr.span, scope),
        ShapeExprKind::Sequence(element) => {
            let lowered = lower_resolved_hir_shape(element, scope);
            LoweredShape {
                shape: Shape::Sequence(Box::new(lowered.shape)),
                diagnostics: lowered.diagnostics,
            }
        }
        ShapeExprKind::Tuple(items) => lower_many(items, scope, Shape::product),
        ShapeExprKind::Optional(inner) => {
            let lowered = lower_resolved_hir_shape(inner, scope);
            LoweredShape {
                shape: Shape::Optional(Box::new(lowered.shape)),
                diagnostics: lowered.diagnostics,
            }
        }
        ShapeExprKind::Union(items) => lower_many(items, scope, Shape::Union),
        ShapeExprKind::Structural(requirements) => lower_structural_shape(requirements, scope),
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

fn lower_structural_shape(
    requirements: &[tune_hir::shape::StructuralShapeRequirement],
    scope: &Scope,
) -> LoweredShape {
    let mut lowered_requirements = Vec::new();
    let mut diagnostics = Vec::new();

    for requirement in requirements {
        match &requirement.kind {
            StructuralShapeRequirementKind::Field { shape } => {
                let shape = shape.as_ref().map(|shape| {
                    let lowered = lower_resolved_hir_shape(shape, scope);
                    diagnostics.extend(lowered.diagnostics);
                    lowered.shape
                });
                lowered_requirements.push(MemberRequirement::Field {
                    name: requirement.name.clone(),
                    shape,
                });
            }
            StructuralShapeRequirementKind::Callable { params, ret } => {
                let mut lowered_params = Vec::new();
                for param in params {
                    let lowered = lower_resolved_hir_shape(param, scope);
                    lowered_params.push(lowered.shape);
                    diagnostics.extend(lowered.diagnostics);
                }
                let ret = ret.as_ref().map(|ret| {
                    let lowered = lower_resolved_hir_shape(ret, scope);
                    diagnostics.extend(lowered.diagnostics);
                    lowered.shape
                });
                lowered_requirements.push(MemberRequirement::Callable {
                    name: requirement.name.clone(),
                    params: lowered_params,
                    ret,
                });
            }
        }
    }

    LoweredShape {
        shape: Shape::Structural(lowered_requirements),
        diagnostics,
    }
}

fn lower_generic_shape(
    name: &str,
    args: &[ShapeExpr],
    span: Option<Span>,
    scope: &Scope,
) -> LoweredShape {
    let (lowered_args, mut diagnostics) = lower_args(args, scope);
    if let Some(shape) = builtin_generic_shape(name, lowered_args.clone()) {
        return LoweredShape { shape, diagnostics };
    }

    match scope.get(name) {
        Some(binding) if matches!(binding.kind, BindingKind::Struct | BindingKind::Enum) => {
            if let Some(diagnostic) =
                generic_arity_diagnostic(name, span, binding.generic_arity, args.len())
            {
                diagnostics.push(diagnostic);
            }
            LoweredShape {
                shape: Shape::Apply {
                    nominal: NominalShape::new(binding.id, name),
                    args: lowered_args,
                },
                diagnostics,
            }
        }
        _ => {
            let span = span.unwrap_or_else(Span::synthetic);
            diagnostics.push(
                Diagnostic::error(
                    codes::UNRESOLVED_NAME,
                    format!("unknown shape `{name}`"),
                    span,
                    "this generic shape name is not in scope",
                )
                .build(),
            );
            LoweredShape {
                shape: Shape::Hole,
                diagnostics,
            }
        }
    }
}

fn lower_args(items: &[ShapeExpr], scope: &Scope) -> (Vec<Shape>, Vec<Diagnostic>) {
    let mut shapes = Vec::new();
    let mut diagnostics = Vec::new();
    for item in items {
        let lowered = lower_resolved_hir_shape(item, scope);
        shapes.push(lowered.shape);
        diagnostics.extend(lowered.diagnostics);
    }

    (shapes, diagnostics)
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

    match scope.get(name) {
        Some(binding) if binding.kind == BindingKind::Struct => {
            let nominal = NominalShape::new(binding.id, name);
            LoweredShape {
                shape: nominal_shape_or_holey_apply(
                    Shape::Struct(nominal.clone()),
                    nominal,
                    binding.generic_arity,
                ),
                diagnostics: Vec::new(),
            }
        }
        Some(binding) if binding.kind == BindingKind::Enum => {
            let nominal = NominalShape::new(binding.id, name);
            LoweredShape {
                shape: nominal_shape_or_holey_apply(
                    Shape::Enum(nominal.clone()),
                    nominal,
                    binding.generic_arity,
                ),
                diagnostics: Vec::new(),
            }
        }
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

fn nominal_shape_or_holey_apply(base: Shape, nominal: NominalShape, generic_arity: usize) -> Shape {
    if generic_arity == 0 {
        base
    } else {
        Shape::Apply {
            nominal,
            args: vec![Shape::Hole; generic_arity],
        }
    }
}

fn generic_arity_diagnostic(
    name: &str,
    span: Option<Span>,
    expected: usize,
    actual: usize,
) -> Option<Diagnostic> {
    (expected != actual).then(|| {
        Diagnostic::error(
            codes::SHAPE_MISMATCH,
            format!("generic shape `{name}` expects {expected} argument(s)"),
            span.unwrap_or_else(Span::synthetic),
            format!("this shape was given {actual} generic argument(s)"),
        )
        .build()
    })
}

fn named_shape(name: &str) -> Shape {
    builtin_shape(name).unwrap_or(Shape::Hole)
}

fn generic_shape(name: &str, args: Vec<Shape>) -> Shape {
    builtin_generic_shape(name, args).unwrap_or(Shape::Hole)
}
