use tune_hir::ExprId;
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::Item;
use tune_resolve::{LocalId, NameTarget, ResolvedModule};

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
    lower_item_with_context(item, None)
}

#[must_use]
pub fn lower_resolved_item_to_plan(item: &Item, resolved: &ResolvedModule) -> Option<PlanFunction> {
    lower_item_with_context(item, Some(resolved))
}

fn lower_item_with_context(item: &Item, resolved: Option<&ResolvedModule>) -> Option<PlanFunction> {
    let body = item.body.as_ref()?;
    let mut plan = PlanFunction {
        name: item
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_owned()),
        ops: Vec::new(),
    };
    let context = LowerContext { resolved };
    context.lower_expr(body, &mut plan.ops);
    Some(plan)
}

struct LowerContext<'a> {
    resolved: Option<&'a ResolvedModule>,
}

impl LowerContext<'_> {
    fn lower_expr(&self, expr: &Expr, ops: &mut Vec<PlanOp>) {
        match &expr.kind {
            ExprKind::Missing | ExprKind::Literal(_) | ExprKind::Name(_) => {}
            ExprKind::CallableValue { params: _, body } => {
                self.lower_expr(body, ops);
                ops.push(PlanOp::CallableValue);
            }
            ExprKind::Sequence(elements) => {
                for element in elements {
                    self.lower_expr(element, ops);
                    ops.push(PlanOp::SequencePush);
                }
            }
            ExprKind::Call { callee, args } => {
                self.lower_expr(callee, ops);
                for arg in args {
                    self.lower_expr(arg, ops);
                }
                ops.push(self.call_op(callee));
            }
            ExprKind::Field { base, name } => {
                self.lower_expr(base, ops);
                ops.push(PlanOp::FieldGet {
                    field: name.clone().unwrap_or_default(),
                });
            }
            ExprKind::Index { base, index } => {
                self.lower_expr(base, ops);
                self.lower_expr(index, ops);
                ops.push(PlanOp::SequenceGet { checked: true });
            }
            ExprKind::Let { value, .. } => {
                if let Some(value) = value {
                    self.lower_expr(value, ops);
                }
                ops.push(PlanOp::LocalLet {
                    local: self.local_for_expr(expr.id),
                });
            }
            ExprKind::Assign { target, value } => {
                self.lower_expr(target, ops);
                self.lower_expr(value, ops);
                ops.push(PlanOp::Assign);
            }
            ExprKind::Unary { op, expr } => {
                self.lower_expr(expr, ops);
                ops.push(PlanOp::UnaryOp { op: *op });
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.lower_expr(lhs, ops);
                self.lower_expr(rhs, ops);
                ops.push(PlanOp::BinaryOp { op: *op });
            }
            ExprKind::Spawn(inner) => {
                self.lower_expr(inner, ops);
                ops.push(PlanOp::Spawn);
            }
            ExprKind::Propagate(inner) => {
                self.lower_expr(inner, ops);
                ops.push(PlanOp::ResultPropagate);
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.lower_expr(&branch.condition, ops);
                    self.lower_expr(&branch.body, ops);
                }
                if let Some(else_branch) = else_branch {
                    self.lower_expr(else_branch, ops);
                }
                ops.push(PlanOp::If);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.lower_expr(scrutinee, ops);
                for arm in arms {
                    self.lower_expr(&arm.body, ops);
                }
                ops.push(PlanOp::Match);
            }
            ExprKind::While { condition, body } => {
                self.lower_expr(condition, ops);
                self.lower_expr(body, ops);
                ops.push(PlanOp::While);
            }
            ExprKind::Loop(body) => {
                self.lower_expr(body, ops);
                ops.push(PlanOp::Loop);
            }
            ExprKind::Break => ops.push(PlanOp::Break),
            ExprKind::Continue => ops.push(PlanOp::Continue),
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.lower_expr(inner, ops);
                }
                ops.push(PlanOp::Return);
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.lower_expr(arg, ops);
                }
                ops.push(PlanOp::Panic);
            }
            ExprKind::For { iterable, body, .. } => {
                self.lower_expr(iterable, ops);
                self.lower_expr(body, ops);
                ops.push(PlanOp::FiniteFor);
            }
            ExprKind::Block(exprs) => {
                for expr in exprs {
                    self.lower_expr(expr, ops);
                }
            }
        }
    }

    fn call_op(&self, callee: &Expr) -> PlanOp {
        match self.name_target(callee.id) {
            Some(NameTarget::TopLevel(target)) => PlanOp::DirectCall { target },
            _ => PlanOp::BoundCall,
        }
    }

    fn name_target(&self, expr: ExprId) -> Option<NameTarget> {
        self.resolved?
            .name_refs
            .iter()
            .find(|name_ref| name_ref.expr == expr)
            .map(|name_ref| name_ref.target)
    }

    fn local_for_expr(&self, expr: ExprId) -> Option<LocalId> {
        self.resolved?
            .locals
            .iter()
            .find(|local| local.expr == Some(expr))
            .map(|local| local.id)
    }
}
