use tune_hir::expr::{Expr, ExprKind};
use tune_resolve::{LocalId, LocalKind, NameTarget};

use super::LowerContext;

impl LowerContext<'_> {
    pub(super) fn captured_locals_in_callable_values(&self, body: &Expr) -> Vec<LocalId> {
        let mut captures = Vec::new();
        collect_callable_value_captures(body, self, &mut captures);
        captures
    }

    pub(super) fn callable_value_captures(&self, body: &Expr) -> Vec<LocalId> {
        let mut declared = Vec::new();
        collect_declared_locals(body, self, &mut declared);

        let mut captures = Vec::new();
        collect_captured_locals(body, self, &declared, &mut captures);
        captures
    }
}

fn collect_callable_value_captures(
    expr: &Expr,
    context: &LowerContext<'_>,
    captures: &mut Vec<LocalId>,
) {
    if let ExprKind::CallableValue { body, .. } = &expr.kind {
        for capture in context.callable_value_captures(body) {
            if !captures.contains(&capture) {
                captures.push(capture);
            }
        }
    }

    walk_expr(expr, &mut |child| {
        collect_callable_value_captures(child, context, captures);
    });
}

fn collect_captured_locals(
    expr: &Expr,
    context: &LowerContext<'_>,
    declared: &[LocalId],
    captures: &mut Vec<LocalId>,
) {
    if let ExprKind::Name(_) = expr.kind
        && let Some(NameTarget::Local(local)) = context.name_target(expr.id)
        && !declared.contains(&local)
        && context.local_kind(local) == Some(LocalKind::Let)
        && !captures.contains(&local)
    {
        captures.push(local);
    }

    walk_expr(expr, &mut |child| {
        collect_captured_locals(child, context, declared, captures);
    });
}

fn collect_declared_locals(expr: &Expr, context: &LowerContext<'_>, declared: &mut Vec<LocalId>) {
    if let ExprKind::Let { .. } = expr.kind
        && let Some(local) = context.local_for_expr(expr.id)
        && !declared.contains(&local)
    {
        declared.push(local);
    }

    walk_expr(expr, &mut |child| {
        collect_declared_locals(child, context, declared);
    });
}

fn walk_expr(expr: &Expr, visit: &mut impl FnMut(&Expr)) {
    match &expr.kind {
        ExprKind::Sequence(items) | ExprKind::Block(items) | ExprKind::Panic(items) => {
            for item in items {
                visit(item);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for field in fields {
                visit(&field.value);
            }
        }
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => visit(body),
        ExprKind::Call { callee, args } => {
            visit(callee);
            for arg in args {
                visit(arg);
            }
        }
        ExprKind::Field { base, .. } => visit(base),
        ExprKind::Index { base, index }
        | ExprKind::Binary {
            lhs: base,
            rhs: index,
            ..
        } => {
            visit(base);
            visit(index);
        }
        ExprKind::Let { value, .. } => {
            if let Some(value) = value {
                visit(value);
            }
        }
        ExprKind::Assign { target, value } => {
            visit(target);
            visit(value);
        }
        ExprKind::Unary { expr, .. } => visit(expr),
        ExprKind::If {
            branches,
            else_branch,
        } => {
            for branch in branches {
                visit(&branch.condition);
                visit(&branch.body);
            }
            if let Some(else_branch) = else_branch {
                visit(else_branch);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            visit(scrutinee);
            for arm in arms {
                visit(&arm.body);
            }
        }
        ExprKind::While { condition, body } => {
            visit(condition);
            visit(body);
        }
        ExprKind::Return(inner) => {
            if let Some(inner) = inner {
                visit(inner);
            }
        }
        ExprKind::For { iterable, body, .. } => {
            visit(iterable);
            visit(body);
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => {}
    }
}
