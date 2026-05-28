use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::shape::ShapeExpr;

use crate::locals::{NameRef, NameTarget};
use crate::prelude::VariantId;

use super::BodyResolver;

impl BodyResolver<'_> {
    pub(super) fn resolve_name_ref(&mut self, name: &str, expr: &Expr) {
        if name == "_" {
            return;
        }

        let target = if name == "self" {
            Some(NameTarget::SelfValue)
        } else {
            self.lookup_local(name).or_else(|| {
                self.resolved
                    .scope
                    .get(name)
                    .map(|binding| NameTarget::TopLevel(binding.id))
                    .or_else(|| self.variant_by_name(name).map(NameTarget::Variant))
            })
        };

        if let Some(target) = target {
            self.resolved.name_refs.push(NameRef {
                expr: expr.id,
                target,
                span: expr.span,
            });
            return;
        }

        self.resolved.diagnostics.push(
            Diagnostic::error(
                codes::UNRESOLVED_NAME,
                format!("unresolved name `{name}`"),
                expr.span.unwrap_or_else(Span::synthetic),
                "this name is not in scope",
            )
            .build(),
        );
    }

    pub(super) fn resolve_struct_name_ref(&mut self, name: &str, expr: &Expr) {
        let Some(binding) = self.resolved.scope.get(name) else {
            self.resolved.diagnostics.push(
                Diagnostic::error(
                    codes::UNRESOLVED_NAME,
                    format!("unresolved struct `{name}`"),
                    expr.span.unwrap_or_else(Span::synthetic),
                    "this struct is not in scope",
                )
                .build(),
            );
            return;
        };

        self.resolved.name_refs.push(NameRef {
            expr: expr.id,
            target: NameTarget::TopLevel(binding.id),
            span: expr.span,
        });
    }

    pub(super) fn variant_by_name(&self, name: &str) -> Option<VariantId> {
        self.resolved
            .variants
            .get(name)
            .or_else(|| self.resolved.prelude.variant(name).map(VariantId::Prelude))
    }

    pub(super) fn resolve_expected_variant_callee(
        &mut self,
        callee: &Expr,
        expected: Option<&ShapeExpr>,
    ) -> bool {
        let ExprKind::Name(name) = &callee.kind else {
            return false;
        };

        self.resolve_expected_variant_name(name, callee, expected)
    }

    pub(super) fn resolve_expected_variant_name(
        &mut self,
        name: &str,
        expr: &Expr,
        expected: Option<&ShapeExpr>,
    ) -> bool {
        let Some(variant) = self.variant_for_expected_enum(name, expected) else {
            return false;
        };

        self.resolved.name_refs.push(NameRef {
            expr: expr.id,
            target: NameTarget::Variant(VariantId::Member(variant)),
            span: expr.span,
        });
        true
    }

    pub(super) fn lookup_local(&self, name: &str) -> Option<NameTarget> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }
}
