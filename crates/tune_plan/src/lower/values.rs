use tune_hir::expr::{Expr, ExprKind, IfBranch};
use tune_shape::{Shape, ShapeAnalysis};

use crate::PlanOp;

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

pub(super) fn if_produces_value(
    branches: &[IfBranch],
    else_branch: Option<&Expr>,
    analysis: Option<&ShapeAnalysis>,
    has_default_else: bool,
) -> bool {
    let branches_produce = branches
        .iter()
        .all(|branch| expr_can_supply_value(&branch.body, analysis));
    if !branches_produce {
        return false;
    }
    else_branch.map_or(has_default_else, |else_branch| {
        expr_can_supply_value(else_branch, analysis)
    })
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
        } => if_produces_value(branches, else_branch.as_deref(), None, false),
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

fn expr_can_supply_value(expr: &Expr, analysis: Option<&ShapeAnalysis>) -> bool {
    expr_produces_value(expr) || expr_is_never(expr, analysis)
}

pub(super) fn default_value_ops(shape: &Shape) -> Option<Vec<PlanOp>> {
    match shape {
        Shape::Int => Some(vec![PlanOp::ConstInt { value: 0 }]),
        Shape::Float => Some(vec![PlanOp::ConstFloat {
            bits: 0.0f64.to_bits(),
        }]),
        Shape::Size => Some(vec![PlanOp::ConstSize { value: 0 }]),
        Shape::Byte => Some(vec![PlanOp::ConstByte { value: 0 }]),
        Shape::Bool => Some(vec![PlanOp::ConstBool { value: false }]),
        Shape::String => Some(vec![PlanOp::ConstString {
            value: String::new(),
        }]),
        Shape::Optional(_) | Shape::Literal(tune_shape::LiteralFact::None) => {
            Some(vec![PlanOp::ConstNone])
        }
        Shape::Sequence(element) => Some(vec![PlanOp::SequenceBuild {
            element_count: 0,
            element_shape: element.as_ref().clone(),
        }]),
        Shape::Tuple(items) => {
            let mut ops = Vec::new();
            for item in items {
                ops.extend(default_value_ops(item)?);
            }
            ops.push(PlanOp::TupleBuild {
                element_count: items.len(),
            });
            Some(ops)
        }
        Shape::Unit => Some(Vec::new()),
        Shape::Hole
        | Shape::Never
        | Shape::Literal(_)
        | Shape::Range(_)
        | Shape::Union(_)
        | Shape::Callable { .. }
        | Shape::Result { .. }
        | Shape::Task(_)
        | Shape::Apply { .. }
        | Shape::Struct(_)
        | Shape::Enum(_)
        | Shape::Structural(_)
        | Shape::Param(_) => None,
    }
}
