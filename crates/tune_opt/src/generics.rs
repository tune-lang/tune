use std::collections::{HashMap, HashSet};

use tune_hir::HirId;
use tune_ir::{IrFunction, IrGenericStrategy, IrOp};
use tune_shape::{MemberRequirement, Shape};

use crate::{OptimizationProfile, OptimizeOptions, Pass, PassReport};

#[must_use]
pub fn run(_function: &mut IrFunction, _options: OptimizeOptions) -> PassReport {
    PassReport {
        pass: Pass::Generics,
        changed: false,
    }
}

#[must_use]
pub fn run_module(functions: &mut [IrFunction], options: OptimizeOptions) -> PassReport {
    let summaries = functions
        .iter()
        .map(FunctionSummary::new)
        .collect::<Vec<_>>();
    let instantiations = instantiation_counts(functions);
    let mut changed = false;
    for function in functions {
        changed |= choose_function_strategies(function, &summaries, &instantiations, options);
    }
    PassReport {
        pass: Pass::Generics,
        changed,
    }
}

#[derive(Debug, Clone)]
struct FunctionSummary {
    owner: Option<HirId>,
    op_count: usize,
}

impl FunctionSummary {
    fn new(function: &IrFunction) -> Self {
        Self {
            owner: function.owner,
            op_count: function
                .blocks
                .iter()
                .map(|block| block.ops.len())
                .sum::<usize>(),
        }
    }
}

fn instantiation_counts(functions: &[IrFunction]) -> HashMap<HirId, usize> {
    let mut seen = HashSet::<(HirId, Vec<Shape>)>::new();
    for function in functions {
        collect_instantiations(function, &mut seen);
    }

    let mut counts = HashMap::<HirId, usize>::new();
    for (target, _) in seen {
        *counts.entry(target).or_default() += 1;
    }
    counts
}

fn collect_instantiations(function: &IrFunction, seen: &mut HashSet<(HirId, Vec<Shape>)>) {
    for op in function.blocks.iter().flat_map(|block| &block.ops) {
        if let IrOp::CallDirect {
            function,
            type_args,
            ..
        } = op
            && !type_args.is_empty()
            && !type_args.iter().any(shape_contains_type_param)
        {
            seen.insert((*function, type_args.clone()));
        }
    }
    for task in &function.task_functions {
        collect_instantiations(task, seen);
    }
}

fn choose_function_strategies(
    function: &mut IrFunction,
    summaries: &[FunctionSummary],
    instantiations: &HashMap<HirId, usize>,
    options: OptimizeOptions,
) -> bool {
    let mut changed = false;
    for op in function
        .blocks
        .iter_mut()
        .flat_map(|block| block.ops.iter_mut())
    {
        if let IrOp::CallDirect {
            function: target,
            type_args,
            generic_strategy,
            ..
        } = op
        {
            let next = strategy_for_call(*target, type_args, summaries, instantiations, options);
            if *generic_strategy != next {
                *generic_strategy = next;
                changed = true;
            }
        }
    }
    for task in &mut function.task_functions {
        changed |= choose_function_strategies(task, summaries, instantiations, options);
    }
    changed
}

fn strategy_for_call(
    target: HirId,
    type_args: &[Shape],
    summaries: &[FunctionSummary],
    instantiations: &HashMap<HirId, usize>,
    options: OptimizeOptions,
) -> IrGenericStrategy {
    if type_args.is_empty() {
        return IrGenericStrategy::None;
    }
    if type_args.iter().any(shape_contains_type_param)
        || options.profile == OptimizationProfile::Debug
    {
        return IrGenericStrategy::WitnessShared;
    }

    let instantiation_count = instantiations.get(&target).copied().unwrap_or(usize::MAX);
    let target_ops = summaries
        .iter()
        .find(|summary| summary.owner == Some(target))
        .map_or(usize::MAX, |summary| summary.op_count);
    if instantiation_count <= options.generic_max_instantiations
        && target_ops <= options.generic_max_ops
    {
        IrGenericStrategy::DirectSpecialization
    } else {
        IrGenericStrategy::WitnessShared
    }
}

fn shape_contains_type_param(shape: &Shape) -> bool {
    match shape {
        Shape::Param(_) => true,
        Shape::Sequence(inner)
        | Shape::Range(inner)
        | Shape::Optional(inner)
        | Shape::Task(inner) => shape_contains_type_param(inner),
        Shape::Tuple(items) | Shape::Union(items) => items.iter().any(shape_contains_type_param),
        Shape::Callable { params, ret } => {
            params.iter().any(shape_contains_type_param) || shape_contains_type_param(ret)
        }
        Shape::Result { ok, err } => {
            shape_contains_type_param(ok) || shape_contains_type_param(err)
        }
        Shape::Apply { args, .. } => args.iter().any(shape_contains_type_param),
        Shape::Structural(requirements) => {
            requirements.iter().any(|requirement| match requirement {
                MemberRequirement::Field { shape, .. } => {
                    shape.as_ref().is_some_and(shape_contains_type_param)
                }
                MemberRequirement::Callable { params, ret, .. } => {
                    params.iter().any(shape_contains_type_param)
                        || ret.as_ref().is_some_and(shape_contains_type_param)
                }
            })
        }
        _ => false,
    }
}
