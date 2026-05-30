use crate::function::{BytecodeCallSite, BytecodeFunction, BytecodeGenericStrategy};

use super::BytecodeValidationError;

pub(super) fn validate_call_generics(
    function_id: u32,
    site: &BytecodeCallSite,
    target: &BytecodeFunction,
) -> Result<(), BytecodeValidationError> {
    let type_arg_count = u32::try_from(site.type_args.len()).map_err(|_| {
        BytecodeValidationError::GenericArgArityMismatch {
            function: function_id,
            target: site.function,
            expected: target.generic_param_count,
            actual: u32::MAX,
        }
    })?;
    if type_arg_count != 0 && type_arg_count != target.generic_param_count {
        return Err(BytecodeValidationError::GenericArgArityMismatch {
            function: function_id,
            target: site.function,
            expected: target.generic_param_count,
            actual: type_arg_count,
        });
    }
    if target.generic_param_count == 0 && site.generic_strategy != BytecodeGenericStrategy::None {
        return Err(BytecodeValidationError::GenericStrategyMismatch {
            function: function_id,
            target: site.function,
        });
    }
    if target.generic_param_count != 0 && type_arg_count == 0 {
        return Err(BytecodeValidationError::GenericArgArityMismatch {
            function: function_id,
            target: site.function,
            expected: target.generic_param_count,
            actual: 0,
        });
    }
    if site.type_args.iter().any(shape_contains_hole) {
        return Err(BytecodeValidationError::UnsolvedGenericArg {
            function: function_id,
            target: site.function,
        });
    }
    if !generic_strategy_allowed(&site.type_args, site.generic_strategy) {
        return Err(BytecodeValidationError::GenericStrategyMismatch {
            function: function_id,
            target: site.function,
        });
    }
    Ok(())
}

fn generic_strategy_allowed(
    type_args: &[tune_shape::Shape],
    strategy: BytecodeGenericStrategy,
) -> bool {
    if type_args.is_empty() {
        return strategy == BytecodeGenericStrategy::None;
    }
    if type_args.iter().any(shape_contains_type_param) {
        strategy == BytecodeGenericStrategy::WitnessShared
    } else {
        matches!(
            strategy,
            BytecodeGenericStrategy::DirectSpecialization | BytecodeGenericStrategy::WitnessShared
        )
    }
}

fn shape_contains_hole(shape: &tune_shape::Shape) -> bool {
    match shape {
        tune_shape::Shape::Hole => true,
        _ => shape_children(shape).any(shape_contains_hole),
    }
}

fn shape_contains_type_param(shape: &tune_shape::Shape) -> bool {
    match shape {
        tune_shape::Shape::Param(_) => true,
        _ => shape_children(shape).any(shape_contains_type_param),
    }
}

fn shape_children(shape: &tune_shape::Shape) -> Box<dyn Iterator<Item = &tune_shape::Shape> + '_> {
    match shape {
        tune_shape::Shape::Sequence(inner)
        | tune_shape::Shape::Range(inner)
        | tune_shape::Shape::Optional(inner)
        | tune_shape::Shape::Task(inner) => Box::new(std::iter::once(inner.as_ref())),
        tune_shape::Shape::Tuple(items) | tune_shape::Shape::Union(items) => Box::new(items.iter()),
        tune_shape::Shape::Callable { params, ret } => {
            Box::new(params.iter().chain(std::iter::once(ret.as_ref())))
        }
        tune_shape::Shape::Result { ok, err } => Box::new([ok.as_ref(), err.as_ref()].into_iter()),
        tune_shape::Shape::Apply { args, .. } => Box::new(args.iter()),
        tune_shape::Shape::Structural(requirements) => Box::new(requirements.iter().flat_map(
            |requirement| match requirement {
                tune_shape::MemberRequirement::Field { shape, .. } => {
                    shape.iter().collect::<Vec<_>>()
                }
                tune_shape::MemberRequirement::Callable { params, ret, .. } => {
                    params.iter().chain(ret.iter()).collect::<Vec<_>>()
                }
            },
        )),
        _ => Box::new(std::iter::empty()),
    }
}
