#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FunctionTarget {
    Item(tune_hir::HirId),
    Member(tune_hir::MemberId),
    Callable(tune_hir::ExprId),
}

pub(crate) fn reachable_functions(
    functions: &[tune_plan::PlanFunction],
    entry: &tune_plan::PlanFunction,
) -> Vec<usize> {
    let mut reachable = Vec::new();
    let mut pending = direct_call_targets(entry).collect::<Vec<_>>();
    pending.reverse();
    while let Some(target) = pending.pop() {
        let Some(index) = functions
            .iter()
            .position(|function| function_matches_target(function, target))
        else {
            continue;
        };
        if reachable.contains(&index) {
            continue;
        }
        reachable.push(index);
        for target in direct_call_targets(&functions[index]) {
            pending.push(target);
        }
    }
    reachable.sort_unstable();
    reachable
}

fn function_matches_target(function: &tune_plan::PlanFunction, target: FunctionTarget) -> bool {
    match target {
        FunctionTarget::Item(item) => function.owner == Some(item) && function.member.is_none(),
        FunctionTarget::Member(member) => function.member == Some(member),
        FunctionTarget::Callable(callable) => function.callable == Some(callable),
    }
}

fn direct_call_targets(
    function: &tune_plan::PlanFunction,
) -> impl Iterator<Item = FunctionTarget> + '_ {
    function.ops.iter().flat_map(direct_call_targets_in_op)
}

fn direct_call_targets_in_op(op: &tune_plan::PlanOp) -> Vec<FunctionTarget> {
    match op {
        tune_plan::PlanOp::DirectCall { target, .. } => Some(FunctionTarget::Item(*target)),
        tune_plan::PlanOp::MemberCall {
            member: Some(member),
            ..
        } => Some(FunctionTarget::Member(*member)),
        tune_plan::PlanOp::Materialize {
            materializer: Some(member),
            ..
        } => Some(FunctionTarget::Member(*member)),
        tune_plan::PlanOp::CallableValue { callable, .. } => {
            Some(FunctionTarget::Callable(*callable))
        }
        _ => None,
    }
    .into_iter()
    .chain(nested_direct_call_targets(op))
    .collect()
}

fn nested_direct_call_targets(op: &tune_plan::PlanOp) -> Vec<FunctionTarget> {
    match op {
        tune_plan::PlanOp::If {
            branches, else_ops, ..
        } => branches
            .iter()
            .flat_map(|branch| {
                branch
                    .condition_ops
                    .iter()
                    .chain(branch.body_ops.iter())
                    .flat_map(direct_call_targets_in_op)
            })
            .chain(else_ops.iter().flat_map(direct_call_targets_in_op))
            .collect(),
        tune_plan::PlanOp::Match { arms, .. } => arms
            .iter()
            .flat_map(|arm| arm.body_ops.iter().flat_map(direct_call_targets_in_op))
            .collect(),
        tune_plan::PlanOp::FiniteFor {
            iterable_ops,
            body_ops,
            contract,
            ..
        } => contract
            .len_member
            .into_iter()
            .chain(contract.index_member)
            .map(FunctionTarget::Member)
            .chain(
                iterable_ops
                    .iter()
                    .chain(body_ops)
                    .flat_map(direct_call_targets_in_op),
            )
            .collect(),
        tune_plan::PlanOp::While {
            condition_ops,
            body_ops,
            ..
        } => condition_ops
            .iter()
            .chain(body_ops)
            .flat_map(direct_call_targets_in_op)
            .collect(),
        tune_plan::PlanOp::Loop { body_ops, .. } => body_ops
            .iter()
            .flat_map(direct_call_targets_in_op)
            .collect(),
        tune_plan::PlanOp::Spawn { body_ops, .. } => body_ops
            .iter()
            .flat_map(direct_call_targets_in_op)
            .collect(),
        tune_plan::PlanOp::BoolAnd {
            lhs_ops, rhs_ops, ..
        }
        | tune_plan::PlanOp::BoolOr {
            lhs_ops, rhs_ops, ..
        } => lhs_ops
            .iter()
            .chain(rhs_ops)
            .flat_map(direct_call_targets_in_op)
            .collect(),
        _ => Vec::new(),
    }
}
