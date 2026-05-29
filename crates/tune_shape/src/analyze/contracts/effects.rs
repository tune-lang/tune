use tune_hir::expr::{Expr, ExprKind};

use crate::BindingKey;

use super::super::Analyzer;

pub(super) fn expr_has_materializer_effect(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Assign { .. }
        | ExprKind::Spawn(_)
        | ExprKind::Propagate(_)
        | ExprKind::Panic(_) => true,
        ExprKind::CallableValue { .. } => false,
        ExprKind::Tuple(elements) | ExprKind::Sequence(elements) | ExprKind::Block(elements) => {
            elements.iter().any(expr_has_materializer_effect)
        }
        ExprKind::Struct { fields, .. } => fields
            .iter()
            .any(|field| expr_has_materializer_effect(&field.value)),
        ExprKind::Call { callee, args } => {
            expr_has_materializer_effect(callee) || args.iter().any(expr_has_materializer_effect)
        }
        ExprKind::Field { base, .. } => expr_has_materializer_effect(base),
        ExprKind::Index { base, index } => {
            expr_has_materializer_effect(base) || expr_has_materializer_effect(index)
        }
        ExprKind::Let { value, .. } => value.as_deref().is_some_and(expr_has_materializer_effect),
        ExprKind::Unary { expr, .. } => expr_has_materializer_effect(expr),
        ExprKind::Binary { lhs, rhs, .. } => {
            expr_has_materializer_effect(lhs) || expr_has_materializer_effect(rhs)
        }
        ExprKind::If {
            branches,
            else_branch,
        } => {
            branches.iter().any(|branch| {
                expr_has_materializer_effect(&branch.condition)
                    || expr_has_materializer_effect(&branch.body)
            }) || else_branch
                .as_deref()
                .is_some_and(expr_has_materializer_effect)
        }
        ExprKind::Match { scrutinee, arms } => {
            expr_has_materializer_effect(scrutinee)
                || arms
                    .iter()
                    .any(|arm| expr_has_materializer_effect(&arm.body))
        }
        ExprKind::While { condition, body } => {
            expr_has_materializer_effect(condition) || expr_has_materializer_effect(body)
        }
        ExprKind::Loop(body) | ExprKind::Return(Some(body)) => expr_has_materializer_effect(body),
        ExprKind::For { iterable, body, .. } => {
            expr_has_materializer_effect(iterable) || expr_has_materializer_effect(body)
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue
        | ExprKind::Return(None) => false,
    }
}

pub(super) fn expr_assigns_binding(
    expr: &Expr,
    source: BindingKey,
    analyzer: &Analyzer<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Assign { target, value } => {
            target_reaches_binding(target, source, analyzer)
                || expr_assigns_binding(value, source, analyzer)
        }
        ExprKind::CallableValue { .. } => false,
        ExprKind::Tuple(elements) | ExprKind::Sequence(elements) | ExprKind::Block(elements) => {
            elements
                .iter()
                .any(|element| expr_assigns_binding(element, source, analyzer))
        }
        ExprKind::Struct { fields, .. } => fields
            .iter()
            .any(|field| expr_assigns_binding(&field.value, source, analyzer)),
        ExprKind::Call { callee, args } => {
            expr_assigns_binding(callee, source, analyzer)
                || args
                    .iter()
                    .any(|arg| expr_assigns_binding(arg, source, analyzer))
        }
        ExprKind::Field { base, .. } => expr_assigns_binding(base, source, analyzer),
        ExprKind::Index { base, index } => {
            expr_assigns_binding(base, source, analyzer)
                || expr_assigns_binding(index, source, analyzer)
        }
        ExprKind::Let { value, .. } => value
            .as_deref()
            .is_some_and(|value| expr_assigns_binding(value, source, analyzer)),
        ExprKind::Unary { expr, .. } => expr_assigns_binding(expr, source, analyzer),
        ExprKind::Binary { lhs, rhs, .. } => {
            expr_assigns_binding(lhs, source, analyzer)
                || expr_assigns_binding(rhs, source, analyzer)
        }
        ExprKind::Spawn(inner) | ExprKind::Propagate(inner) | ExprKind::Loop(inner) => {
            expr_assigns_binding(inner, source, analyzer)
        }
        ExprKind::If {
            branches,
            else_branch,
        } => {
            branches.iter().any(|branch| {
                expr_assigns_binding(&branch.condition, source, analyzer)
                    || expr_assigns_binding(&branch.body, source, analyzer)
            }) || else_branch
                .as_deref()
                .is_some_and(|branch| expr_assigns_binding(branch, source, analyzer))
        }
        ExprKind::Match { scrutinee, arms } => {
            expr_assigns_binding(scrutinee, source, analyzer)
                || arms
                    .iter()
                    .any(|arm| expr_assigns_binding(&arm.body, source, analyzer))
        }
        ExprKind::While { condition, body } => {
            expr_assigns_binding(condition, source, analyzer)
                || expr_assigns_binding(body, source, analyzer)
        }
        ExprKind::Return(inner) => inner
            .as_deref()
            .is_some_and(|inner| expr_assigns_binding(inner, source, analyzer)),
        ExprKind::Panic(args) => args
            .iter()
            .any(|arg| expr_assigns_binding(arg, source, analyzer)),
        ExprKind::For { iterable, body, .. } => {
            expr_assigns_binding(iterable, source, analyzer)
                || expr_assigns_binding(body, source, analyzer)
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => false,
    }
}

fn target_reaches_binding(target: &Expr, source: BindingKey, analyzer: &Analyzer<'_>) -> bool {
    match &target.kind {
        ExprKind::Name(_) => analyzer.binding_key(target) == Some(source),
        ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
            target_reaches_binding(base, source, analyzer)
        }
        _ => false,
    }
}
