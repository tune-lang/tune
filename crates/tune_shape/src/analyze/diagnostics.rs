use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::Expr;
use tune_hir::item::{Item, ItemKind, Visibility};

use super::Analyzer;
use crate::{Shape, expr_propagated_error_shape_fact};

impl Analyzer<'_> {
    pub(super) fn check_public_api_shape(&mut self, item: &Item) {
        if item.visibility != Visibility::Public || item.shape.is_some() {
            return;
        }
        if !matches!(item.kind, ItemKind::Let | ItemKind::CallableDecl) {
            return;
        }
        self.diagnostics.push(
            Diagnostic::warning(
                codes::PUBLIC_API_INFERENCE,
                "public API relies on inferred shape",
                item.span.unwrap_or_else(Span::synthetic),
                "public items need an explicit shape for stable compiler facts",
            )
            .with_help("add an explicit return or storage shape to this public item")
            .build(),
        );
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
