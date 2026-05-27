use tune_ast::AstNode;
use tune_ast::nodes::{Expr as AstExpr, MatchExpr};

use crate::expr::{ExprKind, IfBranch, MatchArm as HirMatchArm};

use super::{ExprLowerer, lower_pattern};

impl ExprLowerer {
    pub(super) fn lower_if(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let mut branches = Vec::new();
        let mut else_branch = None;

        while let Some(condition) = children.next() {
            let Some(body) = children.next() else {
                else_branch = Some(Box::new(self.lower(source, condition)));
                break;
            };

            branches.push(IfBranch {
                condition: self.lower(source, condition),
                body: self.lower(source, body),
            });
        }

        ExprKind::If {
            branches,
            else_branch,
        }
    }

    pub(super) fn lower_match(
        &mut self,
        source: &str,
        expr: AstExpr<'_>,
        node: MatchExpr<'_>,
    ) -> ExprKind {
        let Some(scrutinee) = expr.child_exprs().into_iter().next() else {
            return ExprKind::Missing;
        };

        ExprKind::Match {
            scrutinee: Box::new(self.lower(source, scrutinee)),
            arms: node
                .arms()
                .into_iter()
                .filter_map(|arm| {
                    Some(HirMatchArm {
                        pattern: lower_pattern(source, arm.syntax(), self),
                        body: self.lower(source, arm.expr()?),
                    })
                })
                .collect(),
        }
    }

    pub(super) fn lower_while(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let (Some(condition), Some(body)) = (children.next(), children.next()) else {
            return ExprKind::Missing;
        };

        ExprKind::While {
            condition: Box::new(self.lower(source, condition)),
            body: Box::new(self.lower(source, body)),
        }
    }

    pub(super) fn lower_for(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let (Some(iterable), Some(body)) = (children.next(), children.next()) else {
            return ExprKind::Missing;
        };

        ExprKind::For {
            pattern: lower_pattern(source, expr.syntax(), self),
            iterable: Box::new(self.lower(source, iterable)),
            body: Box::new(self.lower(source, body)),
        }
    }
}
