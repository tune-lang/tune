use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::Expr;
use tune_hir::item::{Item, ItemKind, Visibility};

use super::Analyzer;
use crate::{Shape, expr_propagated_error_shape_fact};

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
        let Some(error_shape) = expr_propagated_error_shape_fact(body, self.module, self.resolved)
        else {
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
        let Some(error_shape) = expr_propagated_error_shape_fact(body, self.module, self.resolved)
        else {
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
