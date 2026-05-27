use tune_hir::expr::{Expr, ExprKind};
use tune_hir::module::Module;
use tune_resolve::ResolvedModule;

use crate::{Shape, expr_shape_fact};

#[must_use]
pub fn expr_result_constructor_shape_fact(
    expr: &Expr,
    module: &Module,
    resolved: &ResolvedModule,
) -> Option<Shape> {
    let mut collector = ResultConstructorCollector::default();
    collector.collect_value(expr, module, resolved);
    collector.finish()
}

#[must_use]
pub fn expr_propagated_error_shape_fact(
    expr: &Expr,
    module: &Module,
    resolved: &ResolvedModule,
) -> Option<Shape> {
    let mut collector = PropagatedErrorCollector::default();
    collector.collect(expr, module, resolved);
    collector.finish()
}

#[derive(Debug, Default)]
struct ResultConstructorCollector {
    ok: Vec<Shape>,
    err: Vec<Shape>,
}

impl ResultConstructorCollector {
    fn collect_value(&mut self, expr: &Expr, module: &Module, resolved: &ResolvedModule) {
        if let Some(shape) = expr_shape_fact(expr, module, resolved) {
            self.add_shape(shape);
            return;
        }

        match &expr.kind {
            ExprKind::Block(exprs) => {
                for expr in exprs {
                    self.collect_return(expr, module, resolved);
                }
                if let Some(last) = exprs.last() {
                    self.collect_value(last, module, resolved);
                }
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.collect_value(&branch.body, module, resolved);
                }
                if let Some(else_branch) = else_branch {
                    self.collect_value(else_branch, module, resolved);
                }
            }
            ExprKind::Match { arms, .. } => {
                for arm in arms {
                    self.collect_value(&arm.body, module, resolved);
                }
            }
            ExprKind::Return(inner) => {
                self.collect_return_expr(inner.as_deref(), module, resolved);
            }
            _ => {}
        }
    }

    fn collect_return(&mut self, expr: &Expr, module: &Module, resolved: &ResolvedModule) {
        match &expr.kind {
            ExprKind::Return(inner) => self.collect_return_expr(inner.as_deref(), module, resolved),
            ExprKind::Block(exprs) => {
                for expr in exprs {
                    self.collect_return(expr, module, resolved);
                }
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.collect_return(&branch.body, module, resolved);
                }
                if let Some(else_branch) = else_branch {
                    self.collect_return(else_branch, module, resolved);
                }
            }
            ExprKind::Match { arms, .. } => {
                for arm in arms {
                    self.collect_return(&arm.body, module, resolved);
                }
            }
            ExprKind::CallableValue { .. } => {}
            _ => {}
        }
    }

    fn collect_return_expr(
        &mut self,
        expr: Option<&Expr>,
        module: &Module,
        resolved: &ResolvedModule,
    ) {
        if let Some(expr) = expr {
            self.collect_value(expr, module, resolved);
        }
    }

    fn add_shape(&mut self, shape: Shape) {
        if let Shape::Result { ok, err } = shape {
            if *ok != Shape::Hole {
                self.ok.push(*ok);
            }
            if *err != Shape::Hole {
                self.err.push(*err);
            }
        }
    }

    fn finish(self) -> Option<Shape> {
        (!self.ok.is_empty() || !self.err.is_empty()).then(|| Shape::Result {
            ok: Box::new(Shape::join_all(self.ok)),
            err: Box::new(Shape::join_all(self.err)),
        })
    }
}

#[derive(Debug, Default)]
struct PropagatedErrorCollector {
    errors: Vec<Shape>,
}

impl PropagatedErrorCollector {
    fn collect(&mut self, expr: &Expr, module: &Module, resolved: &ResolvedModule) {
        match &expr.kind {
            ExprKind::Propagate(inner) => {
                if let Some(Shape::Result { err, .. }) = expr_shape_fact(inner, module, resolved) {
                    self.errors.push(*err);
                }
                self.collect(inner, module, resolved);
            }
            ExprKind::CallableValue { .. } => {}
            ExprKind::Spawn(body) | ExprKind::Loop(body) => self.collect(body, module, resolved),
            ExprKind::Sequence(elements) | ExprKind::Block(elements) => {
                for element in elements {
                    self.collect(element, module, resolved);
                }
            }
            ExprKind::Call { callee, args } => {
                self.collect(callee, module, resolved);
                for arg in args {
                    self.collect(arg, module, resolved);
                }
            }
            ExprKind::Field { base, .. } => self.collect(base, module, resolved),
            ExprKind::Index { base, index } => {
                self.collect(base, module, resolved);
                self.collect(index, module, resolved);
            }
            ExprKind::Let { value, .. } => {
                if let Some(value) = value {
                    self.collect(value, module, resolved);
                }
            }
            ExprKind::Assign { target, value } => {
                self.collect(target, module, resolved);
                self.collect(value, module, resolved);
            }
            ExprKind::Unary { expr, .. } => self.collect(expr, module, resolved),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.collect(lhs, module, resolved);
                self.collect(rhs, module, resolved);
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.collect(&branch.condition, module, resolved);
                    self.collect(&branch.body, module, resolved);
                }
                if let Some(else_branch) = else_branch {
                    self.collect(else_branch, module, resolved);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect(scrutinee, module, resolved);
                for arm in arms {
                    self.collect(&arm.body, module, resolved);
                }
            }
            ExprKind::While { condition, body } => {
                self.collect(condition, module, resolved);
                self.collect(body, module, resolved);
            }
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.collect(inner, module, resolved);
                }
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.collect(arg, module, resolved);
                }
            }
            ExprKind::For { iterable, body, .. } => {
                self.collect(iterable, module, resolved);
                self.collect(body, module, resolved);
            }
            ExprKind::Missing
            | ExprKind::Literal(_)
            | ExprKind::Name(_)
            | ExprKind::Break
            | ExprKind::Continue => {}
        }
    }

    fn finish(self) -> Option<Shape> {
        (!self.errors.is_empty()).then(|| Shape::join_all(self.errors))
    }
}
