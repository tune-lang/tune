use tune_hir::item::Item;
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::ResolvedModule;
use tune_shape::{MemberRequirement, NominalShape, Shape, lower_resolved_hir_shape};

pub(crate) fn lower_shape_expr_for_item(
    shape: &ShapeExpr,
    item: &Item,
    resolved: &ResolvedModule,
) -> Shape {
    match &shape.kind {
        ShapeExprKind::Named(name) if item_has_type_param(item, name) => Shape::Param(name.clone()),
        ShapeExprKind::Named(_) | ShapeExprKind::Missing => {
            lower_resolved_hir_shape(shape, &resolved.scope).shape
        }
        ShapeExprKind::Generic { name, args } => {
            let args = args
                .iter()
                .map(|arg| lower_shape_expr_for_item(arg, item, resolved))
                .collect::<Vec<_>>();
            tune_shape::builtin_generic_shape(name, args.clone()).unwrap_or_else(|| {
                resolved
                    .scope
                    .get(name)
                    .map_or(Shape::Hole, |binding| Shape::Apply {
                        nominal: NominalShape::new(binding.id, name),
                        args,
                    })
            })
        }
        ShapeExprKind::Sequence(inner) => {
            Shape::Sequence(Box::new(lower_shape_expr_for_item(inner, item, resolved)))
        }
        ShapeExprKind::Tuple(items) => Shape::product(
            items
                .iter()
                .map(|item_shape| lower_shape_expr_for_item(item_shape, item, resolved))
                .collect(),
        ),
        ShapeExprKind::Optional(inner) => {
            Shape::Optional(Box::new(lower_shape_expr_for_item(inner, item, resolved)))
        }
        ShapeExprKind::Union(items) => Shape::Union(
            items
                .iter()
                .map(|item_shape| lower_shape_expr_for_item(item_shape, item, resolved))
                .collect(),
        ),
        ShapeExprKind::Callable { params, ret } => Shape::Callable {
            params: params
                .iter()
                .map(|param| lower_shape_expr_for_item(param, item, resolved))
                .collect(),
            ret: Box::new(lower_shape_expr_for_item(ret, item, resolved)),
        },
        ShapeExprKind::Structural(requirements) => Shape::Structural(
            requirements
                .iter()
                .map(|requirement| match &requirement.kind {
                    StructuralShapeRequirementKind::Field { shape } => MemberRequirement::Field {
                        name: requirement.name.clone(),
                        shape: shape
                            .as_ref()
                            .map(|shape| lower_shape_expr_for_item(shape, item, resolved)),
                    },
                    StructuralShapeRequirementKind::Callable { params, ret } => {
                        MemberRequirement::Callable {
                            name: requirement.name.clone(),
                            params: params
                                .iter()
                                .map(|param| lower_shape_expr_for_item(param, item, resolved))
                                .collect(),
                            ret: ret
                                .as_ref()
                                .map(|ret| lower_shape_expr_for_item(ret, item, resolved)),
                        }
                    }
                })
                .collect(),
        ),
    }
}

pub(crate) fn substitute_params(shape: &Shape, subst: &[(String, Shape)]) -> Shape {
    match shape {
        Shape::Param(name) => subst
            .iter()
            .find(|(param, _)| param == name)
            .map_or_else(|| shape.clone(), |(_, value)| value.clone()),
        Shape::Sequence(inner) => Shape::Sequence(Box::new(substitute_params(inner, subst))),
        Shape::Range(inner) => Shape::Range(Box::new(substitute_params(inner, subst))),
        Shape::Tuple(items) => Shape::Tuple(
            items
                .iter()
                .map(|item| substitute_params(item, subst))
                .collect(),
        ),
        Shape::Union(items) => Shape::Union(
            items
                .iter()
                .map(|item| substitute_params(item, subst))
                .collect(),
        ),
        Shape::Optional(inner) => Shape::Optional(Box::new(substitute_params(inner, subst))),
        Shape::Callable { params, ret } => Shape::Callable {
            params: params
                .iter()
                .map(|param| substitute_params(param, subst))
                .collect(),
            ret: Box::new(substitute_params(ret, subst)),
        },
        Shape::Result { ok, err } => Shape::Result {
            ok: Box::new(substitute_params(ok, subst)),
            err: Box::new(substitute_params(err, subst)),
        },
        Shape::Task(inner) => Shape::Task(Box::new(substitute_params(inner, subst))),
        Shape::Apply { nominal, args } => Shape::Apply {
            nominal: nominal.clone(),
            args: args
                .iter()
                .map(|arg| substitute_params(arg, subst))
                .collect(),
        },
        shape => shape.clone(),
    }
}

fn item_has_type_param(item: &Item, name: &str) -> bool {
    item.type_params
        .iter()
        .any(|param| param.name.as_deref() == Some(name))
}
