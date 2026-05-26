use tune_hir::expr::{BinaryOp, Expr, ExprKind, UnaryOp};
use tune_hir::item::Item;

use crate::plan::{PlanFunction, PlanOp};

#[must_use]
pub fn lower_to_plan(name: &str) -> PlanFunction {
    PlanFunction {
        name: name.into(),
        ops: Vec::new(),
    }
}

#[must_use]
pub fn lower_item_to_plan(item: &Item) -> Option<PlanFunction> {
    let body = item.body.as_ref()?;
    let mut plan = PlanFunction {
        name: item
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_owned()),
        ops: Vec::new(),
    };
    lower_expr(body, &mut plan.ops);
    Some(plan)
}

fn lower_expr(expr: &Expr, ops: &mut Vec<PlanOp>) {
    match &expr.kind {
        ExprKind::Missing | ExprKind::Literal(_) | ExprKind::Name(_) => {}
        ExprKind::CallableValue { params: _, body } => {
            lower_expr(body, ops);
            ops.push(PlanOp::CallableValue);
        }
        ExprKind::Sequence(elements) => {
            for element in elements {
                lower_expr(element, ops);
                ops.push(PlanOp::SequencePush);
            }
        }
        ExprKind::Call { callee, args } => {
            lower_expr(callee, ops);
            for arg in args {
                lower_expr(arg, ops);
            }
            ops.push(call_op(callee));
        }
        ExprKind::Field { base, name } => {
            lower_expr(base, ops);
            ops.push(PlanOp::FieldGet {
                field: name.clone().unwrap_or_default(),
            });
        }
        ExprKind::Index { base, index } => {
            lower_expr(base, ops);
            lower_expr(index, ops);
            ops.push(PlanOp::SequenceGet { checked: true });
        }
        ExprKind::Let { name, value, .. } => {
            if let Some(value) = value {
                lower_expr(value, ops);
            }
            ops.push(PlanOp::LocalLet {
                name: name.clone().unwrap_or_default(),
            });
        }
        ExprKind::Assign { target, value } => {
            lower_expr(target, ops);
            lower_expr(value, ops);
            ops.push(PlanOp::Assign);
        }
        ExprKind::Unary { op, expr } => {
            lower_expr(expr, ops);
            ops.push(PlanOp::UnaryOp {
                op: unary_op_name(*op).to_owned(),
            });
        }
        ExprKind::Binary { op, lhs, rhs } => {
            lower_expr(lhs, ops);
            lower_expr(rhs, ops);
            ops.push(PlanOp::BinaryOp {
                op: binary_op_name(*op).to_owned(),
            });
        }
        ExprKind::Spawn(inner) => {
            lower_expr(inner, ops);
            ops.push(PlanOp::Spawn);
        }
        ExprKind::Propagate(inner) => {
            lower_expr(inner, ops);
            ops.push(PlanOp::ResultPropagate);
        }
        ExprKind::Return(inner) => {
            if let Some(inner) = inner {
                lower_expr(inner, ops);
            }
            ops.push(PlanOp::Return);
        }
        ExprKind::For { iterable, body, .. } => {
            lower_expr(iterable, ops);
            lower_expr(body, ops);
            ops.push(PlanOp::FiniteFor);
        }
        ExprKind::Block(exprs) => {
            for expr in exprs {
                lower_expr(expr, ops);
            }
        }
    }
}

fn unary_op_name(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "not",
        UnaryOp::Neg => "-",
        UnaryOp::BitNot => "~",
    }
}

fn binary_op_name(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Is => "is",
        BinaryOp::IsNot => "is not",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "~=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::BitAnd => "&",
        BinaryOp::ShiftLeft => "<<",
        BinaryOp::ShiftRight => ">>",
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
    }
}

fn call_op(callee: &Expr) -> PlanOp {
    match &callee.kind {
        ExprKind::Name(name) => PlanOp::DirectCall {
            function: name.clone(),
        },
        _ => PlanOp::BoundCall,
    }
}
