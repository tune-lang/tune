use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::Item;
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{BindingKind, Scope};

use crate::{
    LoweredShape, MemberRequirement, NominalShape, Shape,
    builtin::{builtin_generic_shape, builtin_shape},
};

pub(super) fn lower_item_shape_expr(shape: &ShapeExpr, item: &Item, scope: &Scope) -> LoweredShape {
    match &shape.kind {
        ShapeExprKind::Missing => LoweredShape {
            shape: Shape::Hole,
            diagnostics: Vec::new(),
        },
        ShapeExprKind::Named(name) if item_has_type_param(item, name) => LoweredShape {
            shape: Shape::Param(name.clone()),
            diagnostics: Vec::new(),
        },
        ShapeExprKind::Named(name) => lower_named_shape(name, shape.span, scope),
        ShapeExprKind::Generic { name, args } => {
            lower_generic_shape(name, args, shape.span, item, scope)
        }
        ShapeExprKind::Sequence(element) => {
            let lowered = lower_item_shape_expr(element, item, scope);
            LoweredShape {
                shape: Shape::Sequence(Box::new(lowered.shape)),
                diagnostics: lowered.diagnostics,
            }
        }
        ShapeExprKind::Tuple(items) => lower_many_item_shapes(items, item, scope, Shape::Tuple),
        ShapeExprKind::Optional(inner) => {
            let lowered = lower_item_shape_expr(inner, item, scope);
            LoweredShape {
                shape: Shape::Optional(Box::new(lowered.shape)),
                diagnostics: lowered.diagnostics,
            }
        }
        ShapeExprKind::Union(items) => lower_many_item_shapes(items, item, scope, Shape::Union),
        ShapeExprKind::Structural(requirements) => {
            lower_structural_shape(requirements, item, scope)
        }
        ShapeExprKind::Callable { params, ret } => {
            let mut lowered_params = Vec::new();
            let mut diagnostics = Vec::new();
            for param in params {
                let lowered = lower_item_shape_expr(param, item, scope);
                lowered_params.push(lowered.shape);
                diagnostics.extend(lowered.diagnostics);
            }
            let lowered_ret = lower_item_shape_expr(ret, item, scope);
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
        _ => unknown_shape(name, span),
    }
}

fn lower_generic_shape(
    name: &str,
    args: &[ShapeExpr],
    span: Option<Span>,
    item: &Item,
    scope: &Scope,
) -> LoweredShape {
    let (lowered_args, mut diagnostics) = lower_item_shape_args(args, item, scope);
    if let Some(shape) = builtin_generic_shape(name, lowered_args.clone()) {
        return LoweredShape { shape, diagnostics };
    }
    match scope.get(name) {
        Some(binding) if matches!(binding.kind, BindingKind::Struct | BindingKind::Enum) => {
            if binding.generic_arity != args.len() {
                diagnostics.push(generic_arity_diagnostic(
                    name,
                    span,
                    binding.generic_arity,
                    args.len(),
                ));
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
            let mut unknown = unknown_shape(name, span);
            diagnostics.append(&mut unknown.diagnostics);
            LoweredShape {
                shape: unknown.shape,
                diagnostics,
            }
        }
    }
}

fn lower_item_shape_args(
    args: &[ShapeExpr],
    item: &Item,
    scope: &Scope,
) -> (Vec<Shape>, Vec<Diagnostic>) {
    let mut shapes = Vec::new();
    let mut diagnostics = Vec::new();
    for arg in args {
        let lowered = lower_item_shape_expr(arg, item, scope);
        shapes.push(lowered.shape);
        diagnostics.extend(lowered.diagnostics);
    }
    (shapes, diagnostics)
}

fn lower_many_item_shapes(
    items: &[ShapeExpr],
    item: &Item,
    scope: &Scope,
    wrap: impl FnOnce(Vec<Shape>) -> Shape,
) -> LoweredShape {
    let (shapes, diagnostics) = lower_item_shape_args(items, item, scope);
    LoweredShape {
        shape: wrap(shapes),
        diagnostics,
    }
}

fn lower_structural_shape(
    requirements: &[tune_hir::shape::StructuralShapeRequirement],
    item: &Item,
    scope: &Scope,
) -> LoweredShape {
    let mut lowered_requirements = Vec::new();
    let mut diagnostics = Vec::new();
    for requirement in requirements {
        match &requirement.kind {
            StructuralShapeRequirementKind::Field { shape } => {
                let shape = shape.as_ref().map(|shape| {
                    let lowered = lower_item_shape_expr(shape, item, scope);
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
                    let lowered = lower_item_shape_expr(param, item, scope);
                    lowered_params.push(lowered.shape);
                    diagnostics.extend(lowered.diagnostics);
                }
                let ret = ret.as_ref().map(|ret| {
                    let lowered = lower_item_shape_expr(ret, item, scope);
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

fn item_has_type_param(item: &Item, name: &str) -> bool {
    item.type_params
        .iter()
        .any(|param| param.name.as_deref() == Some(name))
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

fn unknown_shape(name: &str, span: Option<Span>) -> LoweredShape {
    LoweredShape {
        shape: Shape::Hole,
        diagnostics: vec![
            Diagnostic::error(
                codes::UNRESOLVED_NAME,
                format!("unknown shape `{name}`"),
                span.unwrap_or_else(Span::synthetic),
                "this shape name is not in scope",
            )
            .build(),
        ],
    }
}

fn generic_arity_diagnostic(
    name: &str,
    span: Option<Span>,
    expected: usize,
    actual: usize,
) -> Diagnostic {
    Diagnostic::error(
        codes::SHAPE_MISMATCH,
        format!("generic shape `{name}` expects {expected} argument(s)"),
        span.unwrap_or_else(Span::synthetic),
        format!("this shape was given {actual} generic argument(s)"),
    )
    .build()
}
