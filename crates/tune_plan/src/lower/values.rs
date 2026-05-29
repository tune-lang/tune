use tune_hir::expr::{Expr, ExprKind, IfBranch};

pub(super) fn falls_through(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Return(_) | ExprKind::Panic(_) | ExprKind::Break | ExprKind::Continue => false,
        ExprKind::Block(exprs) => exprs.last().is_none_or(falls_through),
        ExprKind::If {
            branches,
            else_branch: Some(else_branch),
        } => {
            branches.iter().any(|branch| falls_through(&branch.body)) || falls_through(else_branch)
        }
        ExprKind::Loop(body) => falls_through(body),
        _ => true,
    }
}

pub(super) fn if_produces_value(branches: &[IfBranch], else_branch: Option<&Expr>) -> bool {
    let Some(else_branch) = else_branch else {
        return false;
    };
    branches
        .iter()
        .all(|branch| expr_produces_value(&branch.body))
        && expr_produces_value(else_branch)
}

pub(super) fn expr_produces_value(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Missing
        | ExprKind::Let { .. }
        | ExprKind::Assign { .. }
        | ExprKind::While { .. }
        | ExprKind::Loop(_)
        | ExprKind::Break
        | ExprKind::Continue
        | ExprKind::Return(_)
        | ExprKind::Panic(_)
        | ExprKind::For { .. } => false,
        ExprKind::Block(exprs) => exprs.last().is_some_and(expr_produces_value),
        ExprKind::If {
            branches,
            else_branch,
        } => if_produces_value(branches, else_branch.as_deref()),
        ExprKind::Literal(_)
        | ExprKind::Tuple(_)
        | ExprKind::Sequence(_)
        | ExprKind::Struct { .. }
        | ExprKind::Name(_)
        | ExprKind::CallableValue { .. }
        | ExprKind::Call { .. }
        | ExprKind::Field { .. }
        | ExprKind::Index { .. }
        | ExprKind::Unary { .. }
        | ExprKind::Binary { .. }
        | ExprKind::Spawn(_)
        | ExprKind::Propagate(_)
        | ExprKind::Match { .. } => true,
    }
}

pub(super) fn task_join_base<'expr>(callee: &'expr Expr, args: &[Expr]) -> Option<&'expr Expr> {
    if !args.is_empty() {
        return None;
    }

    let ExprKind::Field { base, name } = &callee.kind else {
        return None;
    };

    matches!(name.as_deref(), Some("join")).then_some(base)
}
