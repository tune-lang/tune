use tune_hir::expr::{Expr, ExprKind};
use tune_hir::shape::ShapeExpr;

use super::BodyResolver;

impl BodyResolver<'_> {
    pub(super) fn resolve_statement_names_with_return_expected(
        &mut self,
        expr: &Expr,
        return_expected: Option<&ShapeExpr>,
    ) {
        match &expr.kind {
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.resolve_expr_names_with_expected(inner, return_expected);
                }
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.resolve_expr_names(&branch.condition);
                    self.with_scope(|this| {
                        this.resolve_statement_names_with_return_expected(
                            &branch.body,
                            return_expected,
                        );
                    });
                }
                if let Some(else_branch) = else_branch {
                    self.with_scope(|this| {
                        this.resolve_statement_names_with_return_expected(
                            else_branch,
                            return_expected,
                        );
                    });
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.resolve_expr_names(scrutinee);
                let scrutinee_expected = self.expected_shape_for_expr(scrutinee);
                for arm in arms {
                    self.validate_match_pattern(&arm.pattern);
                    self.with_scope(|this| {
                        this.bind_pattern_names_with_expected(
                            &arm.pattern,
                            scrutinee_expected.as_ref(),
                        );
                        this.resolve_statement_names_with_return_expected(
                            &arm.body,
                            return_expected,
                        );
                    });
                }
            }
            ExprKind::While { condition, body } => {
                self.resolve_expr_names(condition);
                self.with_scope(|this| {
                    this.resolve_statement_names_with_return_expected(body, return_expected);
                });
            }
            ExprKind::Loop(body) => {
                self.with_scope(|this| {
                    this.resolve_statement_names_with_return_expected(body, return_expected);
                });
            }
            ExprKind::For {
                pattern,
                iterable,
                body,
            } => {
                self.resolve_expr_names(iterable);
                self.with_scope(|this| {
                    this.bind_pattern_names(pattern);
                    this.resolve_statement_names_with_return_expected(body, return_expected);
                });
            }
            ExprKind::Block(exprs) => {
                self.with_scope(|this| {
                    for expr in exprs {
                        this.resolve_statement_names_with_return_expected(expr, return_expected);
                    }
                });
            }
            _ => self.resolve_expr_names(expr),
        }
    }
}
