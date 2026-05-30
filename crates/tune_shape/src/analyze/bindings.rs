use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};

use super::{Analyzer, AssignmentCheck};
use crate::{
    BindingKey, BindingState, LiteralFact, Shape, can_materialize, expr_literal_fact,
    lower_resolved_hir_shape,
};

impl Analyzer<'_> {
    pub(super) fn analyze_let(
        &mut self,
        expr: &Expr,
        shape: Option<&tune_hir::shape::ShapeExpr>,
        value: Option<&Expr>,
    ) -> Shape {
        let declared = shape.map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope));
        let expected = declared
            .as_ref()
            .map_or(Shape::Hole, |shape| shape.shape.clone());
        if let Some(declared) = declared {
            self.diagnostics.extend(declared.diagnostics);
        }

        let actual = value
            .map(|value| self.analyze_expr_expected(value, &expected))
            .unwrap_or_else(|| default_current_shape(&expected));
        if let Some(value) = value {
            if matches!(value.kind, ExprKind::Sequence(_)) {
                let materializer = self.sequence_materializer(&expected);
                self.check_materializer(&expected, value.span);
                if materializer.is_none() {
                    self.check_value_against(&expected, &actual, value.span);
                }
            } else {
                self.check_value_against(&expected, &actual, value.span);
            }
            if shape.is_none() {
                self.check_unannotated_optional_copy(&actual, value.span);
            }
        }

        if let Some(local) = self.local_for_expr(expr.id) {
            let literal = value.and_then(expr_literal_fact);
            let binding = literal.map_or_else(
                || {
                    BindingState::new(
                        BindingKey::Local(local),
                        self.local_name(local),
                        expected.clone(),
                        initial_current_shape(&expected, &actual),
                        expr.span,
                    )
                },
                |literal| {
                    let storage_shape = if expected == Shape::Hole {
                        if literal.is_numeric() {
                            Shape::Hole
                        } else {
                            literal.storage_shape()
                        }
                    } else {
                        expected.clone()
                    };
                    let current_shape = initial_literal_current_shape(&storage_shape, &literal);
                    let binding = BindingState::literal(
                        BindingKey::Local(local),
                        self.local_name(local),
                        storage_shape,
                        literal,
                        expr.span,
                    );
                    binding.with_committed_current(current_shape)
                },
            );
            self.frame.define(binding);
        }

        actual
    }

    pub(super) fn analyze_assign(&mut self, target: &Expr, value: &Expr) -> Shape {
        let actual = self.analyze_expr(value);
        if let Some(key) = self.binding_key(target) {
            if self.reject_const_assignment(key, target.span) {
                return actual;
            }
            let expected = self.assignment_expected_shape(key, &actual);
            self.check_value_against(&expected, &actual, value.span);
            self.assignments.push(AssignmentCheck {
                target: key,
                expected: expected.clone(),
                actual: actual.clone(),
                span: target.span,
            });
            self.frame.assign_shape(key, actual.clone());
        } else {
            self.analyze_expr(target);
        }
        actual
    }

    fn reject_const_assignment(&mut self, key: BindingKey, span: Option<Span>) -> bool {
        match key {
            BindingKey::Param(_) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        codes::INVALID_ASSIGNMENT_TARGET,
                        "cannot assign to parameter binding",
                        span.unwrap_or_else(Span::synthetic),
                        "callable parameters are const bindings",
                    )
                    .with_help("introduce a local binding if you need a new value")
                    .build(),
                );
                true
            }
            BindingKey::TopLevel(item)
                if self.module.items.iter().any(|candidate| {
                    candidate.id == item && candidate.kind == tune_hir::item::ItemKind::CallableDecl
                }) =>
            {
                self.diagnostics.push(
                    Diagnostic::error(
                        codes::INVALID_ASSIGNMENT_TARGET,
                        "cannot assign to stable callable declaration",
                        span.unwrap_or_else(Span::synthetic),
                        "`let name(args) = ...` declares a stable callable identity",
                    )
                    .with_help("use `let name = _(args) = ...` for an assignable callable value")
                    .build(),
                );
                true
            }
            _ => false,
        }
    }

    fn assignment_expected_shape(&mut self, key: BindingKey, actual: &Shape) -> Shape {
        let Some(binding) = self.frame.get(key) else {
            return Shape::Hole;
        };
        if binding.storage_shape != Shape::Hole {
            return binding.storage_shape.clone();
        }
        let Some(literal) = binding.literal_fact.as_ref() else {
            return Shape::Hole;
        };
        if !literal.is_numeric() {
            return literal.storage_shape();
        }
        if let Shape::Literal(next) = actual
            && next.is_numeric()
        {
            let solved = if matches!(next, LiteralFact::Numeric { text } if text.contains('.')) {
                Shape::Float
            } else {
                Shape::Int
            };
            if let Some(binding) = self.frame.get_mut(key) {
                binding.storage_shape = solved.clone();
            }
            return solved;
        }
        if matches!(
            actual,
            Shape::Int | Shape::Float | Shape::Size | Shape::Byte
        ) {
            if let Some(binding) = self.frame.get_mut(key) {
                binding.storage_shape = actual.clone();
            }
            return actual.clone();
        }
        Shape::Int
    }

    pub(super) fn check_value_against(
        &mut self,
        expected: &Shape,
        actual: &Shape,
        span: Option<Span>,
    ) {
        if !expected.accepts(actual) {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::ASSIGNMENT_SHAPE_MISMATCH,
                    "assigned value does not match storage shape",
                    span.unwrap_or_else(Span::synthetic),
                    format!("expected `{expected:?}`, got `{actual:?}`"),
                )
                .build(),
            );
        }
    }

    pub(super) fn check_unannotated_optional_copy(&mut self, actual: &Shape, span: Option<Span>) {
        match actual {
            Shape::Literal(LiteralFact::None) => self.diagnostics.push(
                Diagnostic::error(
                    codes::SHAPE_MISMATCH,
                    "optional value is proven none",
                    span.unwrap_or_else(Span::synthetic),
                    "this unannotated binding would copy an absent value",
                )
                .with_help("narrow with `is not none` before using the present value")
                .build(),
            ),
            Shape::Optional(_) => self.diagnostics.push(
                Diagnostic::warning(
                    codes::SHAPE_MISMATCH,
                    "optional value may be none",
                    span.unwrap_or_else(Span::synthetic),
                    "this unannotated binding copies a value that may be absent",
                )
                .with_help("narrow with `is not none` if this binding needs the present value")
                .build(),
            ),
            _ => {}
        }
    }
}

fn default_current_shape(expected: &Shape) -> Shape {
    if matches!(expected, Shape::Optional(_)) {
        Shape::Literal(LiteralFact::None)
    } else {
        Shape::Hole
    }
}

fn initial_current_shape(expected: &Shape, actual: &Shape) -> Shape {
    match (expected, actual) {
        (Shape::Hole, actual) => actual.clone(),
        (Shape::Optional(_), Shape::Literal(LiteralFact::None)) => {
            Shape::Literal(LiteralFact::None)
        }
        (Shape::Optional(inner), actual) if inner.accepts(actual) => actual.clone(),
        (expected, _) => expected.clone(),
    }
}

fn initial_literal_current_shape(storage: &Shape, literal: &LiteralFact) -> Shape {
    match (storage, literal) {
        (Shape::Optional(_), LiteralFact::None) => Shape::Literal(LiteralFact::None),
        (Shape::Optional(inner), literal) if can_materialize(literal, inner) => {
            inner.as_ref().clone()
        }
        (Shape::Hole, _) => Shape::Literal(literal.clone()),
        (storage, _) => storage.clone(),
    }
}
