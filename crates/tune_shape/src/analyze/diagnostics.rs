use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart};
use tune_hir::item::{Item, ItemKind, Visibility};

use super::Analyzer;
use crate::Shape;

impl Analyzer<'_> {
    pub(super) fn check_public_api_shape(&mut self, item: &Item) {
        if item.visibility != Visibility::Public {
            return;
        }

        match item.kind {
            ItemKind::Let if item.shape.is_none() => {
                self.diagnostics.push(public_api_diag(
                    item,
                    "public value has inferred storage shape",
                    "add an explicit storage shape to this public value",
                    ["storage shape is inferred".to_owned()],
                ));
            }
            ItemKind::CallableDecl => {
                let mut inferred = Vec::new();
                if item.shape.is_none() {
                    inferred.push("return shape is inferred".to_owned());
                }
                inferred.extend(
                    item.params
                        .iter()
                        .filter(|param| param.shape.is_none())
                        .map(|param| {
                            format!("parameter `{}` shape is inferred", param_name(param))
                        }),
                );
                if !inferred.is_empty() {
                    self.diagnostics.push(public_api_diag(
                        item,
                        "public callable has inferred signature shape",
                        "add explicit parameter and return shapes to make this public callable stable",
                        inferred,
                    ));
                }
            }
            _ => {}
        }
    }

    pub(super) fn check_result_propagation(&mut self, item: &Item, body: &Expr, expected: &Shape) {
        let Some(error_shape) = self.propagated_error_shape(body) else {
            return;
        };
        let Shape::Result { err, .. } = expected else {
            self.diagnostics
                .push(result_propagation_diag(item, body, &error_shape));
            return;
        };
        if !err.accepts(&error_shape) {
            self.diagnostics
                .push(result_propagation_diag(item, body, &error_shape));
        }
    }

    pub(super) fn check_untyped_result_propagation(&mut self, item: &Item, body: &Expr) {
        let Some(error_shape) = self.propagated_error_shape(body) else {
            return;
        };
        self.diagnostics
            .push(result_propagation_diag(item, body, &error_shape));
    }

    pub(super) fn check_returns_against(&mut self, expected: &Shape) {
        for returned in &self.returns {
            if expected.accepts(&returned.shape) {
                continue;
            }
            self.diagnostics.push(
                Diagnostic::error(
                    codes::ASSIGNMENT_SHAPE_MISMATCH,
                    "returned value does not match callable return shape",
                    returned.span.unwrap_or_else(Span::synthetic),
                    format!("expected `{expected:?}`, got `{:?}`", returned.shape),
                )
                .build(),
            );
        }
    }

    fn propagated_error_shape(&self, expr: &Expr) -> Option<Shape> {
        let mut errors = Vec::new();
        self.collect_propagated_errors(expr, &mut errors);
        (!errors.is_empty()).then(|| Shape::join_all(errors))
    }

    fn collect_propagated_errors(&self, expr: &Expr, errors: &mut Vec<Shape>) {
        match &expr.kind {
            ExprKind::Propagate(inner) => {
                if let Some(Shape::Result { err, .. }) = self.recorded_expr_shape(inner) {
                    errors.push(*err);
                }
                self.collect_propagated_errors(inner, errors);
            }
            ExprKind::CallableValue { .. } => {}
            ExprKind::Spawn(body) | ExprKind::Loop(body) => {
                self.collect_propagated_errors(body, errors);
            }
            ExprKind::Tuple(elements)
            | ExprKind::Sequence(elements)
            | ExprKind::Block(elements) => {
                for element in elements {
                    self.collect_propagated_errors(element, errors);
                }
            }
            ExprKind::Struct { fields, .. } => {
                for field in fields {
                    self.collect_propagated_errors(&field.value, errors);
                }
            }
            ExprKind::Call { callee, args } => {
                self.collect_propagated_errors(callee, errors);
                for arg in args {
                    self.collect_propagated_errors(arg, errors);
                }
            }
            ExprKind::Field { base, .. } => self.collect_propagated_errors(base, errors),
            ExprKind::Index { base, index } => {
                self.collect_propagated_errors(base, errors);
                self.collect_propagated_errors(index, errors);
            }
            ExprKind::Let { value, .. } => {
                if let Some(value) = value {
                    self.collect_propagated_errors(value, errors);
                }
            }
            ExprKind::Assign { target, value } => {
                self.collect_propagated_errors(target, errors);
                self.collect_propagated_errors(value, errors);
            }
            ExprKind::Unary { expr, .. } => self.collect_propagated_errors(expr, errors),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.collect_propagated_errors(lhs, errors);
                self.collect_propagated_errors(rhs, errors);
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.collect_propagated_errors(&branch.condition, errors);
                    self.collect_propagated_errors(&branch.body, errors);
                }
                if let Some(else_branch) = else_branch {
                    self.collect_propagated_errors(else_branch, errors);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect_propagated_errors(scrutinee, errors);
                for arm in arms {
                    self.collect_propagated_errors(&arm.body, errors);
                }
            }
            ExprKind::While { condition, body } => {
                self.collect_propagated_errors(condition, errors);
                self.collect_propagated_errors(body, errors);
            }
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.collect_propagated_errors(inner, errors);
                }
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.collect_propagated_errors(arg, errors);
                }
            }
            ExprKind::For { iterable, body, .. } => {
                self.collect_propagated_errors(iterable, errors);
                self.collect_propagated_errors(body, errors);
            }
            ExprKind::Literal(LiteralKind::String(literal)) => {
                for part in &literal.parts {
                    if let StringPart::Interpolation(expr) = part {
                        self.collect_propagated_errors(expr, errors);
                    }
                }
            }
            ExprKind::Missing
            | ExprKind::Literal(_)
            | ExprKind::Name(_)
            | ExprKind::Break
            | ExprKind::Continue => {}
        }
    }

    fn recorded_expr_shape(&self, expr: &Expr) -> Option<Shape> {
        self.expr_shapes
            .iter()
            .rev()
            .find(|shape| shape.expr == expr.id)
            .map(|shape| shape.shape.clone())
    }
}

fn public_api_diag(
    item: &Item,
    title: &'static str,
    help: &'static str,
    inferred: impl IntoIterator<Item = String>,
) -> tune_diagnostics::Diagnostic {
    Diagnostic::warning(
        codes::PUBLIC_API_INFERENCE,
        title,
        item.span.unwrap_or_else(Span::synthetic),
        "this public API surface depends on inference",
    )
    .with_fact("inferred public surface", inferred)
    .with_help(help)
    .build()
}

fn param_name(param: &tune_hir::item::Param) -> &str {
    param.name.as_deref().unwrap_or("_")
}

fn result_propagation_diag(item: &Item, body: &Expr, error_shape: &Shape) -> Diagnostic {
    Diagnostic::error(
        codes::RESULT_PROPAGATION_ERROR,
        "`!` propagates an error shape not carried by the return shape",
        body.span.or(item.span).unwrap_or_else(Span::synthetic),
        format!("propagated error shape is `{error_shape:?}`"),
    )
    .with_help("return a `Result<Ok, Error>` shape that can carry this error")
    .build()
}
