use super::CallSignature;
use tune_hir::item::Item;

use crate::{MemberRequirement, Shape};

type GenericSolution = Vec<(String, Shape)>;

pub(super) fn solve_generic_call_signature(
    signature: CallSignature,
    args: &[Shape],
    expected_ret: Option<&Shape>,
) -> CallSignature {
    let mut solved = Vec::new();
    for (param, arg) in signature.params.iter().zip(args) {
        collect_generic_shape_constraints(param, arg, &mut solved);
    }
    if let Some(expected) = expected_ret {
        collect_generic_shape_constraints(&signature.ret, expected, &mut solved);
    }

    if solved.is_empty() {
        return signature;
    }

    CallSignature {
        params: signature
            .params
            .iter()
            .map(|shape| substitute_generic_params(shape, &solved))
            .collect(),
        ret: substitute_generic_params(&signature.ret, &solved),
        ..signature
    }
}

pub(super) fn item_type_param_solution(item: &Item, args: &[Shape]) -> GenericSolution {
    item.type_params
        .iter()
        .zip(args)
        .filter_map(|(param, arg)| Some((param.name.clone()?, arg.clone())))
        .collect()
}

pub(super) fn collect_generic_shape_constraints(
    expected: &Shape,
    actual: &Shape,
    solved: &mut GenericSolution,
) {
    match (expected, actual) {
        (Shape::Param(name), actual) => merge_generic_solution(name, actual.clone(), solved),
        (Shape::Sequence(expected), Shape::Sequence(actual))
        | (Shape::Range(expected), Shape::Range(actual))
        | (Shape::Optional(expected), Shape::Optional(actual))
        | (Shape::Task(expected), Shape::Task(actual)) => {
            collect_generic_shape_constraints(expected, actual, solved);
        }
        (Shape::Tuple(expected_items), Shape::Tuple(actual_items))
            if expected_items.len() == actual_items.len() =>
        {
            for (expected, actual) in expected_items.iter().zip(actual_items) {
                collect_generic_shape_constraints(expected, actual, solved);
            }
        }
        (
            Shape::Callable { params, ret },
            Shape::Callable {
                params: actual_params,
                ret: actual_ret,
            },
        ) if params.len() == actual_params.len() => {
            for (expected, actual) in params.iter().zip(actual_params) {
                collect_generic_shape_constraints(expected, actual, solved);
            }
            collect_generic_shape_constraints(ret, actual_ret, solved);
        }
        (
            Shape::Result { ok, err },
            Shape::Result {
                ok: actual_ok,
                err: actual_err,
            },
        ) => {
            collect_generic_shape_constraints(ok, actual_ok, solved);
            collect_generic_shape_constraints(err, actual_err, solved);
        }
        (
            Shape::Apply { nominal, args },
            Shape::Apply {
                nominal: actual_nominal,
                args: actual_args,
            },
        ) if nominal.same_identity(actual_nominal) && args.len() == actual_args.len() => {
            for (expected, actual) in args.iter().zip(actual_args) {
                collect_generic_shape_constraints(expected, actual, solved);
            }
        }
        _ => {}
    }
}

pub(super) fn shape_has_type_params(shape: &Shape) -> bool {
    match shape {
        Shape::Param(_) => true,
        Shape::Sequence(inner)
        | Shape::Range(inner)
        | Shape::Optional(inner)
        | Shape::Task(inner) => shape_has_type_params(inner),
        Shape::Tuple(items) | Shape::Union(items) => items.iter().any(shape_has_type_params),
        Shape::Callable { params, ret } => {
            params.iter().any(shape_has_type_params) || shape_has_type_params(ret)
        }
        Shape::Result { ok, err } => shape_has_type_params(ok) || shape_has_type_params(err),
        Shape::Apply { args, .. } => args.iter().any(shape_has_type_params),
        Shape::Structural(requirements) => {
            requirements.iter().any(|requirement| match requirement {
                MemberRequirement::Field { shape, .. } => {
                    shape.as_ref().is_some_and(shape_has_type_params)
                }
                MemberRequirement::Callable { params, ret, .. } => {
                    params.iter().any(shape_has_type_params)
                        || ret.as_ref().is_some_and(shape_has_type_params)
                }
            })
        }
        _ => false,
    }
}

fn merge_generic_solution(name: &str, shape: Shape, solved: &mut GenericSolution) {
    if let Some((_, existing)) = solved.iter_mut().find(|(param, _)| param == name) {
        if shape.accepts(existing) {
            *existing = shape;
        } else if !existing.accepts(&shape) {
            *existing = existing.clone().join(shape);
        }
    } else {
        solved.push((name.to_owned(), shape));
    }
}

pub(super) fn substitute_generic_params(shape: &Shape, solved: &[(String, Shape)]) -> Shape {
    match shape {
        Shape::Param(name) => solved
            .iter()
            .rev()
            .find(|(param, _)| param == name)
            .map_or(Shape::Hole, |(_, shape)| shape.clone()),
        Shape::Sequence(inner) => {
            Shape::Sequence(Box::new(substitute_generic_params(inner, solved)))
        }
        Shape::Range(inner) => Shape::Range(Box::new(substitute_generic_params(inner, solved))),
        Shape::Optional(inner) => {
            Shape::Optional(Box::new(substitute_generic_params(inner, solved)))
        }
        Shape::Callable { params, ret } => Shape::Callable {
            params: params
                .iter()
                .map(|param| substitute_generic_params(param, solved))
                .collect(),
            ret: Box::new(substitute_generic_params(ret, solved)),
        },
        Shape::Result { ok, err } => Shape::Result {
            ok: Box::new(substitute_generic_params(ok, solved)),
            err: Box::new(substitute_generic_params(err, solved)),
        },
        Shape::Task(inner) => Shape::Task(Box::new(substitute_generic_params(inner, solved))),
        Shape::Tuple(items) => Shape::Tuple(
            items
                .iter()
                .map(|item| substitute_generic_params(item, solved))
                .collect(),
        ),
        Shape::Union(items) => Shape::Union(
            items
                .iter()
                .map(|item| substitute_generic_params(item, solved))
                .collect(),
        ),
        Shape::Apply { nominal, args } => Shape::Apply {
            nominal: nominal.clone(),
            args: args
                .iter()
                .map(|arg| substitute_generic_params(arg, solved))
                .collect(),
        },
        Shape::Structural(requirements) => Shape::Structural(requirements.clone()),
        shape => shape.clone(),
    }
}
