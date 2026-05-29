use super::CallSignature;
use crate::Shape;

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

fn collect_generic_shape_constraints(
    expected: &Shape,
    actual: &Shape,
    solved: &mut Vec<(String, Shape)>,
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

fn merge_generic_solution(name: &str, shape: Shape, solved: &mut Vec<(String, Shape)>) {
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

fn substitute_generic_params(shape: &Shape, solved: &[(String, Shape)]) -> Shape {
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
