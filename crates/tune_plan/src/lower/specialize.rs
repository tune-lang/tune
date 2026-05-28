use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::Item;
use tune_hir::module::Module;
use tune_resolve::{NameTarget, ResolvedModule};
use tune_shape::Shape;

use super::LowerContext;

pub(super) fn infer_direct_call_param_shapes(
    module: &Module,
    resolved: &ResolvedModule,
) -> Vec<(tune_hir::MemberId, Shape)> {
    let mut inferred = Vec::new();
    for item in &module.items {
        let Some(body) = item.body.as_ref() else {
            continue;
        };
        let analysis = tune_shape::analyze_item(module, resolved, item);
        let context = LowerContext {
            resolved: Some(resolved),
            module: Some(module),
            analysis: Some(&analysis),
            self_shape: None,
            struct_escape: crate::StructEscapeReason::Local,
            structural_witnesses: Vec::new(),
            param_shapes: Vec::new(),
            captured_locals: Vec::new(),
        };
        collect_direct_call_param_shapes(body, &context, module, &mut inferred);
    }
    consistent_param_shapes(inferred)
}

fn consistent_param_shapes(
    inferred: Vec<(tune_hir::MemberId, Shape)>,
) -> Vec<(tune_hir::MemberId, Shape)> {
    let mut consistent = Vec::<(tune_hir::MemberId, Shape)>::new();
    let mut conflicts = Vec::<tune_hir::MemberId>::new();

    for (param, shape) in inferred {
        if conflicts.contains(&param) {
            continue;
        }
        if let Some((_, existing)) = consistent
            .iter()
            .find(|(existing_param, _)| *existing_param == param)
        {
            if *existing != shape {
                conflicts.push(param);
                consistent.retain(|(existing_param, _)| *existing_param != param);
            }
        } else {
            consistent.push((param, shape));
        }
    }

    consistent
}

fn collect_direct_call_param_shapes(
    expr: &Expr,
    context: &LowerContext<'_>,
    module: &Module,
    inferred: &mut Vec<(tune_hir::MemberId, Shape)>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            collect_call_arg_shapes(callee, args, context, module, inferred);
            collect_direct_call_param_shapes(callee, context, module, inferred);
            for arg in args {
                collect_direct_call_param_shapes(arg, context, module, inferred);
            }
        }
        ExprKind::Sequence(items) | ExprKind::Block(items) | ExprKind::Panic(items) => {
            for item in items {
                collect_direct_call_param_shapes(item, context, module, inferred);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for field in fields {
                collect_direct_call_param_shapes(&field.value, context, module, inferred);
            }
        }
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => collect_direct_call_param_shapes(body, context, module, inferred),
        ExprKind::Field { base, .. } => {
            collect_direct_call_param_shapes(base, context, module, inferred);
        }
        ExprKind::Index { base, index }
        | ExprKind::Binary {
            lhs: base,
            rhs: index,
            ..
        } => {
            collect_direct_call_param_shapes(base, context, module, inferred);
            collect_direct_call_param_shapes(index, context, module, inferred);
        }
        ExprKind::Let { value, .. } => {
            if let Some(value) = value {
                collect_direct_call_param_shapes(value, context, module, inferred);
            }
        }
        ExprKind::Assign { target, value } => {
            collect_direct_call_param_shapes(target, context, module, inferred);
            collect_direct_call_param_shapes(value, context, module, inferred);
        }
        ExprKind::Unary { expr, .. } => {
            collect_direct_call_param_shapes(expr, context, module, inferred);
        }
        ExprKind::If {
            branches,
            else_branch,
        } => {
            for branch in branches {
                collect_direct_call_param_shapes(&branch.condition, context, module, inferred);
                collect_direct_call_param_shapes(&branch.body, context, module, inferred);
            }
            if let Some(else_branch) = else_branch {
                collect_direct_call_param_shapes(else_branch, context, module, inferred);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_direct_call_param_shapes(scrutinee, context, module, inferred);
            for arm in arms {
                collect_direct_call_param_shapes(&arm.body, context, module, inferred);
            }
        }
        ExprKind::While { condition, body } => {
            collect_direct_call_param_shapes(condition, context, module, inferred);
            collect_direct_call_param_shapes(body, context, module, inferred);
        }
        ExprKind::Return(inner) => {
            if let Some(inner) = inner {
                collect_direct_call_param_shapes(inner, context, module, inferred);
            }
        }
        ExprKind::For { iterable, body, .. } => {
            collect_direct_call_param_shapes(iterable, context, module, inferred);
            collect_direct_call_param_shapes(body, context, module, inferred);
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => {}
    }
}

fn collect_call_arg_shapes(
    callee: &Expr,
    args: &[Expr],
    context: &LowerContext<'_>,
    module: &Module,
    inferred: &mut Vec<(tune_hir::MemberId, Shape)>,
) {
    let ExprKind::Name(_) = callee.kind else {
        return;
    };
    let Some(NameTarget::TopLevel(target)) = context.name_target(callee.id) else {
        return;
    };
    let Some(item) = module.items.iter().find(|item| item.id == target) else {
        return;
    };
    collect_param_arg_shapes(item, args, context, inferred);
}

fn collect_param_arg_shapes(
    item: &Item,
    args: &[Expr],
    context: &LowerContext<'_>,
    inferred: &mut Vec<(tune_hir::MemberId, Shape)>,
) {
    for (param, arg) in item.params.iter().zip(args) {
        if param.shape.is_none()
            && let Some(shape) = context.expr_shape(arg)
            && shape != Shape::Hole
        {
            inferred.push((param.id, shape));
        }
    }
}
