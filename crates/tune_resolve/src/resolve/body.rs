use std::collections::HashSet;

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::Item;
use tune_hir::pattern::{Pattern, PatternKind};

use super::ResolvedModule;

pub(super) fn resolve_item_body(resolved: &mut ResolvedModule, item: &Item) {
    let Some(body) = &item.body else {
        return;
    };
    let mut locals = item
        .params
        .iter()
        .filter_map(|param| param.name.clone())
        .collect::<HashSet<_>>();

    resolve_expr_names(resolved, body, &mut locals);
}

fn resolve_expr_names(resolved: &mut ResolvedModule, expr: &Expr, locals: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::Missing | ExprKind::Literal(_) => {}
        ExprKind::Sequence(elements) => {
            for element in elements {
                resolve_expr_names(resolved, element, locals);
            }
        }
        ExprKind::Name(name) => resolve_name_ref(resolved, name, expr.span, locals),
        ExprKind::CallableValue { params, body } => {
            let outer = locals.clone();
            for param in params {
                if let Some(name) = &param.name {
                    locals.insert(name.clone());
                }
            }
            resolve_expr_names(resolved, body, locals);
            *locals = outer;
        }
        ExprKind::Call { callee, args } => {
            resolve_expr_names(resolved, callee, locals);
            for arg in args {
                resolve_expr_names(resolved, arg, locals);
            }
        }
        ExprKind::Field { base, .. } => resolve_expr_names(resolved, base, locals),
        ExprKind::Index { base, index } => {
            resolve_expr_names(resolved, base, locals);
            resolve_expr_names(resolved, index, locals);
        }
        ExprKind::Let { name, value, .. } => {
            if let Some(value) = value {
                resolve_expr_names(resolved, value, locals);
            }
            if let Some(name) = name {
                locals.insert(name.clone());
            }
        }
        ExprKind::Assign { target, value } => {
            resolve_expr_names(resolved, target, locals);
            resolve_expr_names(resolved, value, locals);
        }
        ExprKind::Spawn(inner) | ExprKind::Propagate(inner) => {
            resolve_expr_names(resolved, inner, locals);
        }
        ExprKind::Return(inner) => {
            if let Some(inner) = inner {
                resolve_expr_names(resolved, inner, locals);
            }
        }
        ExprKind::For {
            pattern,
            iterable,
            body,
        } => {
            resolve_expr_names(resolved, iterable, locals);
            let outer = locals.clone();
            bind_pattern_names(pattern, locals);
            resolve_expr_names(resolved, body, locals);
            *locals = outer;
        }
        ExprKind::Block(exprs) => {
            let outer = locals.clone();
            for expr in exprs {
                resolve_expr_names(resolved, expr, locals);
            }
            *locals = outer;
        }
    }
}

fn resolve_name_ref(
    resolved: &mut ResolvedModule,
    name: &str,
    span: Option<Span>,
    locals: &HashSet<String>,
) {
    if name == "self" || locals.contains(name) || resolved.scope.get(name).is_some() {
        return;
    }

    resolved.diagnostics.push(
        Diagnostic::error(
            codes::UNRESOLVED_NAME,
            format!("unresolved name `{name}`"),
            span.unwrap_or_else(Span::synthetic),
            "this name is not in scope",
        )
        .build(),
    );
}

fn bind_pattern_names(pattern: &Pattern, locals: &mut HashSet<String>) {
    match &pattern.kind {
        PatternKind::Binding(name) => {
            locals.insert(name.clone());
        }
        PatternKind::Tuple(patterns) => {
            for pattern in patterns {
                bind_pattern_names(pattern, locals);
            }
        }
        PatternKind::Variant { args, .. } => {
            for pattern in args {
                bind_pattern_names(pattern, locals);
            }
        }
        PatternKind::Hole
        | PatternKind::Unit
        | PatternKind::StructuralShape
        | PatternKind::Else => {}
    }
}
