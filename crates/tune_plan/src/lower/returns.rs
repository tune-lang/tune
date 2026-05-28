use tune_hir::expr::{Expr, ExprKind};

use super::{LowerContext, PlanIfBranch, PlanMatchArm, PlanOp, StructEscapeReason};
use crate::lower::values::{expr_produces_value, if_produces_value};

impl LowerContext<'_> {
    pub(super) fn lower_return_expr(&self, expr: &Expr, ops: &mut Vec<PlanOp>) {
        match &expr.kind {
            ExprKind::Block(exprs) => {
                if let Some((last, leading)) = exprs.split_last() {
                    for expr in leading {
                        self.lower_expr(expr, ops);
                    }
                    self.lower_return_expr(last, ops);
                }
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                ops.push(PlanOp::If {
                    branches: branches
                        .iter()
                        .map(|branch| PlanIfBranch {
                            condition: branch.condition.id,
                            body: branch.body.id,
                            condition_ops: self.lower_expr_to_ops(&branch.condition),
                            body_ops: self.lower_return_expr_to_ops(&branch.body),
                        })
                        .collect(),
                    else_body: else_branch.as_ref().map(|branch| branch.id),
                    else_ops: else_branch
                        .as_ref()
                        .map_or_else(Vec::new, |branch| self.lower_return_expr_to_ops(branch)),
                    produces_value: if_produces_value(branches, else_branch.as_deref()),
                    span: expr.span,
                });
            }
            ExprKind::Match { scrutinee, arms } => {
                if self.lower_structural_return_match(scrutinee, arms, ops) {
                    return;
                }
                self.lower_expr(scrutinee, ops);
                ops.push(PlanOp::Match {
                    scrutinee: scrutinee.id,
                    arms: arms
                        .iter()
                        .map(|arm| PlanMatchArm {
                            pattern: arm.pattern.clone(),
                            body: arm.body.id,
                            variant: self.pattern_variant(&arm.pattern),
                            bindings: self.pattern_bindings(&arm.pattern),
                            body_ops: self.lower_return_expr_to_ops(&arm.body),
                        })
                        .collect(),
                    produces_value: arms.iter().all(|arm| expr_produces_value(&arm.body)),
                    span: expr.span,
                });
            }
            ExprKind::Return(_) => self.lower_expr(expr, ops),
            ExprKind::Struct { .. } => self
                .with_struct_escape(StructEscapeReason::Returned)
                .lower_expr(expr, ops),
            _ => self.lower_expr(expr, ops),
        }
    }

    fn lower_return_expr_to_ops(&self, expr: &Expr) -> Vec<PlanOp> {
        let mut ops = Vec::new();
        self.lower_return_expr(expr, &mut ops);
        ops
    }
}
