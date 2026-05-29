use tune_hir::expr::{Expr, ExprKind, IfBranch};
use tune_shape::{Shape, ShapeAnalysis};

pub(super) fn falls_through(expr: &Expr, analysis: Option<&ShapeAnalysis>) -> bool {
    if expr_is_never(expr, analysis) {
        return false;
    }
    match &expr.kind {
        ExprKind::Return(_) | ExprKind::Panic(_) | ExprKind::Break | ExprKind::Continue => false,
        ExprKind::Block(exprs) => exprs
            .last()
            .is_none_or(|expr| falls_through(expr, analysis)),
        ExprKind::If {
            branches,
            else_branch: Some(else_branch),
        } => {
            branches
                .iter()
                .any(|branch| falls_through(&branch.body, analysis))
                || falls_through(else_branch, analysis)
        }
        ExprKind::Loop(body) => falls_through(body, analysis),
        _ => true,
    }
}

fn expr_is_never(expr: &Expr, analysis: Option<&ShapeAnalysis>) -> bool {
    analysis.is_some_and(|analysis| {
        analysis
            .expr_shapes
            .iter()
            .rev()
            .find(|shape| shape.expr == expr.id)
            .is_some_and(|shape| shape.shape == Shape::Never)
    })
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
