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
        ExprKind::Spawn(inner) | ExprKind::Propagate(inner) => {
            resolve_expr_names(resolved, inner, locals);
        }
        ExprKind::For {
            pattern,
            iterable,
            body,
        } => {
            resolve_expr_names(resolved, iterable, locals);
            let added = bind_pattern_names(pattern, locals);
            resolve_expr_names(resolved, body, locals);
            for name in added {
                locals.remove(&name);
            }
        }
        ExprKind::Block(exprs) => {
            for expr in exprs {
                resolve_expr_names(resolved, expr, locals);
            }
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

fn bind_pattern_names(pattern: &Pattern, locals: &mut HashSet<String>) -> Vec<String> {
    match &pattern.kind {
        PatternKind::Binding(name) => {
            locals.insert(name.clone());
            vec![name.clone()]
        }
        PatternKind::Tuple(patterns) => patterns
            .iter()
            .flat_map(|pattern| bind_pattern_names(pattern, locals))
            .collect(),
        PatternKind::Variant { args, .. } => args
            .iter()
            .flat_map(|pattern| bind_pattern_names(pattern, locals))
            .collect(),
        PatternKind::Hole
        | PatternKind::Unit
        | PatternKind::StructuralShape
        | PatternKind::Else => Vec::new(),
    }
}
