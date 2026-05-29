use tune_hir::expr::{Expr, ExprKind, LiteralKind};
use tune_hir::item::Item;
use tune_hir::module::Module;
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{NameTarget, PreludeVariant, ResolvedModule, VariantId};

use crate::{LiteralFact, MemberRequirement, Shape};

#[must_use]
pub fn expr_literal_fact(expr: &Expr) -> Option<LiteralFact> {
    match &expr.kind {
        ExprKind::Literal(literal) => literal_fact(literal),
        ExprKind::Sequence(elements) => elements
            .iter()
            .map(expr_literal_fact)
            .collect::<Option<Vec<_>>>()
            .map(|elements| LiteralFact::Sequence { elements }),
        ExprKind::Tuple(elements) => elements
            .iter()
            .map(expr_literal_fact)
            .collect::<Option<Vec<_>>>()
            .map(|elements| LiteralFact::Tuple { elements }),
        _ => None,
    }
}

fn literal_fact(literal: &LiteralKind) -> Option<LiteralFact> {
    match literal {
        LiteralKind::Int(text) | LiteralKind::Float(text) => {
            Some(LiteralFact::Numeric { text: text.clone() })
        }
        LiteralKind::String(literal) => Some(LiteralFact::String {
            segments: literal
                .parts
                .iter()
                .map(|part| match part {
                    tune_hir::expr::StringPart::Text(text)
                    | tune_hir::expr::StringPart::Interpolation(text) => text.clone(),
                })
                .collect(),
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
            variant_shape(target, &[], module)
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
    let arg_shapes = args
        .iter()
        .map(|arg| value_shape_hint(arg, module, resolved))
        .collect::<Vec<_>>();

    variant_shape(name_target(callee, resolved)?, &arg_shapes, module)
}

fn value_shape_hint(expr: &Expr, module: &Module, resolved: &ResolvedModule) -> Shape {
    expr_shape_fact(expr, module, resolved).unwrap_or_else(|| {
        expr_literal_fact(expr)
            .map(Shape::Literal)
            .unwrap_or(Shape::Hole)
    })
}

fn name_target(expr: &Expr, resolved: &ResolvedModule) -> Option<NameTarget> {
    resolved
        .name_refs
        .iter()
        .find(|name_ref| name_ref.expr == expr.id)
        .map(|name_ref| name_ref.target)
}

fn variant_shape(target: NameTarget, arg_shapes: &[Shape], module: &Module) -> Option<Shape> {
    match target {
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Ok)) => {
            Some(Shape::Result {
                ok: Box::new(first_arg_shape(arg_shapes)),
                err: Box::new(Shape::Hole),
            })
        }
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Error)) => {
            Some(Shape::Result {
                ok: Box::new(Shape::Hole),
                err: Box::new(first_arg_shape(arg_shapes)),
            })
        }
        NameTarget::Variant(VariantId::Member(variant)) => {
            member_variant_shape(variant, arg_shapes, module)
        }
        _ => None,
    }
}

fn first_arg_shape(arg_shapes: &[Shape]) -> Shape {
    arg_shapes.first().cloned().unwrap_or(Shape::Hole)
}

fn member_variant_shape(
    variant_id: tune_hir::MemberId,
    arg_shapes: &[Shape],
    module: &Module,
) -> Option<Shape> {
    let item = module
        .items
        .iter()
        .find(|item| item.id == variant_id.owner)?;
    let name = item.name.as_ref()?;
    if item.type_params.is_empty() {
        return Some(Shape::Enum(name.clone()));
    }

    let variant = item
        .variants
        .iter()
        .find(|variant| variant.id == variant_id)?;
    let mut solved = item
        .type_params
        .iter()
        .filter_map(|param| Some((param.name.clone()?, Shape::Hole)))
        .collect::<Vec<_>>();

    for (payload, arg) in variant.payload.iter().zip(arg_shapes) {
        collect_type_param_shapes(payload, arg, item, &mut solved);
    }

    Some(Shape::Apply {
        name: name.clone(),
        args: solved.into_iter().map(|(_, shape)| shape).collect(),
    })
}

fn collect_type_param_shapes(
    payload: &ShapeExpr,
    arg: &Shape,
    item: &Item,
    solved: &mut [(String, Shape)],
) {
    collect_shape_params(&lower_declared_shape(payload, item), arg, solved);
}

fn collect_shape_params(payload: &Shape, arg: &Shape, solved: &mut [(String, Shape)]) {
    match (payload, arg) {
        (Shape::Param(name), arg) => {
            if let Some((_, existing)) = solved.iter_mut().find(|(param, _)| param == name) {
                *existing = existing.clone().join(arg.clone());
            }
        }
        (Shape::Sequence(payload), Shape::Sequence(arg))
        | (Shape::Optional(payload), Shape::Optional(arg)) => {
            collect_shape_params(payload, arg, solved);
        }
        (Shape::Tuple(payload_items), Shape::Tuple(arg_items)) => {
            for (payload, arg) in payload_items.iter().zip(arg_items) {
                collect_shape_params(payload, arg, solved);
            }
        }
        (Shape::Union(payload_items), arg) => {
            for payload in payload_items {
                collect_shape_params(payload, arg, solved);
            }
        }
        (
            Shape::Apply { name, args },
            Shape::Apply {
                name: arg_name,
                args: arg_args,
            },
        ) if name == arg_name => {
            for (payload, arg) in args.iter().zip(arg_args) {
                collect_shape_params(payload, arg, solved);
            }
        }
        _ => {}
    }
}

fn lower_declared_shape(expr: &ShapeExpr, item: &Item) -> Shape {
    match &expr.kind {
        ShapeExprKind::Named(name)
            if item
                .type_params
                .iter()
                .any(|param| param.name.as_deref() == Some(name.as_str())) =>
        {
            Shape::Param(name.clone())
        }
        ShapeExprKind::Named(name) => named_shape_hint(name),
        ShapeExprKind::Sequence(element) => {
            Shape::Sequence(Box::new(lower_declared_shape(element, item)))
        }
        ShapeExprKind::Tuple(items) => Shape::Tuple(
            items
                .iter()
                .map(|item_shape| lower_declared_shape(item_shape, item))
                .collect(),
        ),
        ShapeExprKind::Optional(inner) => {
            Shape::Optional(Box::new(lower_declared_shape(inner, item)))
        }
        ShapeExprKind::Union(items) => Shape::Union(
            items
                .iter()
                .map(|item_shape| lower_declared_shape(item_shape, item))
                .collect(),
        ),
        ShapeExprKind::Structural(requirements) => Shape::Structural(
            requirements
                .iter()
                .map(|requirement| match &requirement.kind {
                    StructuralShapeRequirementKind::Field { shape } => MemberRequirement::Field {
                        name: requirement.name.clone(),
                        shape: shape
                            .as_ref()
                            .map(|shape| lower_declared_shape(shape, item)),
                    },
                    StructuralShapeRequirementKind::Callable { params, ret } => {
                        MemberRequirement::Callable {
                            name: requirement.name.clone(),
                            params: params
                                .iter()
                                .map(|param| lower_declared_shape(param, item))
                                .collect(),
                            ret: ret.as_ref().map(|ret| lower_declared_shape(ret, item)),
                        }
                    }
                })
                .collect(),
        ),
        ShapeExprKind::Generic { name, args } => Shape::Apply {
            name: name.clone(),
            args: args
                .iter()
                .map(|arg| lower_declared_shape(arg, item))
                .collect(),
        },
        ShapeExprKind::Callable { params, ret } => Shape::Callable {
            params: params
                .iter()
                .map(|param| lower_declared_shape(param, item))
                .collect(),
            ret: Box::new(lower_declared_shape(ret, item)),
        },
        ShapeExprKind::Missing => Shape::Hole,
    }
}

fn named_shape_hint(name: &str) -> Shape {
    match name {
        "Never" => Shape::Never,
        "()" | "Unit" => Shape::Unit,
        "Int" => Shape::Int,
        "Float" => Shape::Float,
        "Size" => Shape::Size,
        "Byte" => Shape::Byte,
        "Bool" => Shape::Bool,
        "String" => Shape::String,
        _ => Shape::Hole,
    }
}

fn result_ok_shape(shape: Shape) -> Option<Shape> {
    match shape {
        Shape::Result { ok, .. } => Some(*ok),
        _ => None,
    }
}
