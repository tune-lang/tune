use std::borrow::Cow;

use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart};
use tune_hir::item::Item;
use tune_hir::module::Module;
use tune_hir::shape::{ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{NameTarget, ResolvedModule};
use tune_shape::Shape;

use super::LowerContext;

pub(super) fn infer_direct_call_param_shapes_from_analyses(
    module: &Module,
    resolved: &ResolvedModule,
    analyses: &[tune_shape::ShapeAnalysis],
) -> Vec<(tune_hir::MemberId, Shape)> {
    let mut inferred = Vec::new();
    for (item, analysis) in module.items.iter().zip(analyses) {
        let Some(body) = item.body.as_ref() else {
            continue;
        };
        let context = LowerContext {
            resolved: Some(resolved),
            module: Some(module),
            analysis: Some(analysis),
            self_shape: None,
            struct_escape: crate::StructEscapeReason::Local,
            param_shapes: Cow::Borrowed(&[]),
            captured_locals: Cow::Borrowed(&[]),
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
        ExprKind::Tuple(items)
        | ExprKind::Sequence(items)
        | ExprKind::Block(items)
        | ExprKind::Panic(items) => {
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
        ExprKind::Literal(LiteralKind::String(literal)) => {
            for part in &literal.parts {
                if let StringPart::Interpolation(expr) = part {
                    collect_direct_call_param_shapes(expr, context, module, inferred);
                }
            }
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
        if param_is_specializable(item, param.shape.as_ref())
            && let Some(shape) = context.expr_shape(arg)
            && shape != Shape::Hole
        {
            inferred.push((param.id, shape));
        }
    }
}

fn param_is_specializable(item: &Item, shape: Option<&tune_hir::shape::ShapeExpr>) -> bool {
    let Some(shape) = shape else {
        return true;
    };
    let ShapeExprKind::Named(name) = &shape.kind else {
        return false;
    };
    item.type_params
        .iter()
        .find(|param| param.name.as_deref() == Some(name.as_str()))
        .and_then(|param| param.constraint.as_ref())
        .is_some_and(shape_expr_is_structural)
}

fn shape_expr_is_structural(shape: &tune_hir::shape::ShapeExpr) -> bool {
    match &shape.kind {
        ShapeExprKind::Structural(requirements) => requirements.iter().all(|requirement| {
            matches!(
                requirement.kind,
                StructuralShapeRequirementKind::Field { .. }
                    | StructuralShapeRequirementKind::Callable { .. }
            )
        }),
        _ => false,
    }
}
