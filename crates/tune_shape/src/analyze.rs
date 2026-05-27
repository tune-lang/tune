use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};
mod contracts;
mod control;

use tune_hir::item::{Item, ItemKind, Visibility};
use tune_hir::module::Module;
use tune_hir::{ExprId, MemberId};
use tune_resolve::ResolvedModule;

use crate::{
    BindingKey, BindingState, Shape, StateFrame, expr_literal_fact,
    expr_propagated_error_shape_fact, expr_shape_fact, lower_resolved_hir_shape,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprShape {
    pub expr: ExprId,
    pub shape: Shape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignmentCheck {
    pub target: BindingKey,
    pub expected: Shape,
    pub actual: Shape,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteForCheck {
    pub iterable: ExprId,
    pub len_member: Option<MemberId>,
    pub index_member: Option<MemberId>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializerCheck {
    pub target: Shape,
    pub materializer: Option<MemberId>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeAnalysis {
    pub frame: StateFrame,
    pub expr_shapes: Vec<ExprShape>,
    pub assignments: Vec<AssignmentCheck>,
    pub finite_for: Vec<FiniteForCheck>,
    pub materializers: Vec<MaterializerCheck>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn analyze_item(module: &Module, resolved: &ResolvedModule, item: &Item) -> ShapeAnalysis {
    let mut analyzer = Analyzer {
        module,
        resolved,
        frame: StateFrame::new(),
        expr_shapes: Vec::new(),
        assignments: Vec::new(),
        finite_for: Vec::new(),
        materializers: Vec::new(),
        diagnostics: Vec::new(),
    };
    analyzer.seed_item(item);
    analyzer.check_public_api_shape(item);
    if let Some(body) = &item.body {
        let actual = analyzer.analyze_expr(body);
        if let Some(shape) = &item.shape {
            let lowered = lower_resolved_hir_shape(shape, &resolved.scope);
            let expected = lowered.shape;
            analyzer.diagnostics.extend(lowered.diagnostics);
            analyzer.check_result_propagation(item, body, &expected);
            if matches!(body.kind, ExprKind::Sequence(_)) {
                let materializer = analyzer.sequence_materializer(&expected);
                analyzer.check_materializer(&expected, body.span);
                if materializer.is_none() {
                    analyzer.check_value_against(&expected, &actual, body.span);
                }
            } else {
                analyzer.check_value_against(&expected, &actual, body.span);
            }
        } else {
            analyzer.check_untyped_result_propagation(item, body);
        }
    }
    analyzer.finish()
}

#[must_use]
pub fn analyze_module(module: &Module, resolved: &ResolvedModule) -> Vec<ShapeAnalysis> {
    module
        .items
        .iter()
        .map(|item| analyze_item(module, resolved, item))
        .collect()
}

struct Analyzer<'a> {
    module: &'a Module,
    resolved: &'a ResolvedModule,
    frame: StateFrame,
    expr_shapes: Vec<ExprShape>,
    assignments: Vec<AssignmentCheck>,
    finite_for: Vec<FiniteForCheck>,
    materializers: Vec<MaterializerCheck>,
    diagnostics: Vec<Diagnostic>,
}

impl Analyzer<'_> {
    fn finish(self) -> ShapeAnalysis {
        ShapeAnalysis {
            frame: self.frame,
            expr_shapes: self.expr_shapes,
            assignments: self.assignments,
            finite_for: self.finite_for,
            materializers: self.materializers,
            diagnostics: self.diagnostics,
        }
    }

    fn seed_item(&mut self, item: &Item) {
        for param in &item.params {
            let shape = param
                .shape
                .as_ref()
                .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope))
                .map_or(Shape::Hole, |lowered| {
                    self.diagnostics.extend(lowered.diagnostics);
                    lowered.shape
                });
            self.frame.define(BindingState::new(
                BindingKey::Param(param.id),
                param.name.clone(),
                shape.clone(),
                shape,
                param.span,
            ));
        }
    }

    fn analyze_expr(&mut self, expr: &Expr) -> Shape {
        let shape = match &expr.kind {
            ExprKind::Missing => Shape::Hole,
            ExprKind::Literal(_) | ExprKind::Sequence(_) => self.literal_or_sequence_shape(expr),
            ExprKind::Name(_) => self.name_shape(expr),
            ExprKind::Call { callee, args } => {
                self.analyze_expr(callee);
                for arg in args {
                    self.analyze_expr(arg);
                }
                expr_shape_fact(expr, self.module, self.resolved).unwrap_or(Shape::Hole)
            }
            ExprKind::Let { shape, value, .. } => {
                self.analyze_let(expr, shape.as_ref(), value.as_deref())
            }
            ExprKind::Assign { target, value } => self.analyze_assign(target, value),
            ExprKind::If {
                branches,
                else_branch,
            } => self.analyze_if(branches, else_branch.as_deref()),
            ExprKind::Match { scrutinee, arms } => self.analyze_match(expr, scrutinee, arms),
            ExprKind::For {
                pattern,
                iterable,
                body,
            } => self.analyze_for(expr, pattern, iterable, body),
            ExprKind::Block(exprs) => exprs
                .iter()
                .map(|expr| self.analyze_expr(expr))
                .last()
                .unwrap_or(Shape::Unit),
            ExprKind::Propagate(inner) => {
                let inner = self.analyze_expr(inner);
                match inner {
                    Shape::Result { ok, .. } => *ok,
                    _ => Shape::Hole,
                }
            }
            ExprKind::Return(inner) => inner
                .as_deref()
                .map(|inner| self.analyze_expr(inner))
                .unwrap_or(Shape::Never),
            ExprKind::Panic(args) => {
                for arg in args {
                    self.analyze_expr(arg);
                }
                Shape::Never
            }
            ExprKind::Spawn(inner) => Shape::Task(Box::new(self.analyze_expr(inner))),
            ExprKind::Field { base, .. } => {
                self.analyze_expr(base);
                Shape::Hole
            }
            ExprKind::Index { base, index } => {
                self.analyze_expr(base);
                self.analyze_expr(index);
                Shape::Hole
            }
            ExprKind::CallableValue { body, .. } => {
                self.analyze_expr(body);
                Shape::Hole
            }
            ExprKind::Loop(body) => self.analyze_loop(body),
            ExprKind::While { condition, body } => self.analyze_while(condition, body),
            ExprKind::Unary { expr, .. } => self.analyze_expr(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.analyze_expr(lhs);
                self.analyze_expr(rhs);
                Shape::Bool
            }
            ExprKind::Break | ExprKind::Continue => Shape::Never,
        };
        self.record_expr_shape(expr.id, shape.clone());
        shape
    }

    fn literal_or_sequence_shape(&mut self, expr: &Expr) -> Shape {
        if let ExprKind::Sequence(elements) = &expr.kind {
            for element in elements {
                self.analyze_expr(element);
            }
        }
        expr_literal_fact(expr).map_or(Shape::Hole, Shape::Literal)
    }

    fn analyze_let(
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
            .map(|value| self.analyze_expr(value))
            .unwrap_or(Shape::Hole);
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
        }

        if let Some(local) = self.local_for_expr(expr.id) {
            let literal = value.and_then(expr_literal_fact);
            let binding = literal.map_or_else(
                || {
                    BindingState::new(
                        BindingKey::Local(local),
                        self.local_name(local),
                        expected.clone(),
                        if expected == Shape::Hole {
                            actual.clone()
                        } else {
                            expected.clone()
                        },
                        expr.span,
                    )
                },
                |literal| {
                    BindingState::literal(
                        BindingKey::Local(local),
                        self.local_name(local),
                        expected.clone(),
                        literal,
                        expr.span,
                    )
                },
            );
            self.frame.define(binding);
        }

        actual
    }

    fn analyze_assign(&mut self, target: &Expr, value: &Expr) -> Shape {
        let actual = self.analyze_expr(value);
        if let Some(key) = self.binding_key(target) {
            let expected = self
                .frame
                .get(key)
                .map_or(Shape::Hole, |binding| binding.storage_shape.clone());
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

    fn check_public_api_shape(&mut self, item: &Item) {
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

    fn check_result_propagation(&mut self, item: &Item, body: &Expr, expected: &Shape) {
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

    fn check_untyped_result_propagation(&mut self, item: &Item, body: &Expr) {
        let Some(error_shape) = expr_propagated_error_shape_fact(body, self.module, self.resolved)
        else {
            return;
        };
        self.diagnostics
            .push(result_propagation_diag(item, body, &error_shape));
    }

    fn check_value_against(&mut self, expected: &Shape, actual: &Shape, span: Option<Span>) {
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
