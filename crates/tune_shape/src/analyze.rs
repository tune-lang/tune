use crate::LiteralFact;
use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart, UnaryOp};
mod callable;
mod calls;
mod contracts;
mod control;
mod diagnostics;
mod fields;
mod operators;

use tune_hir::item::{Item, ItemKind};
use tune_hir::module::Module;
use tune_hir::shape::{ShapeExpr, ShapeExprKind};
use tune_hir::{ExprId, MemberId};
use tune_resolve::{BindingKind, ResolvedModule, VariantId};

use crate::{
    BindingKey, BindingState, Shape, StateFrame, expr_literal_fact, lower_resolved_hir_shape,
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
    pub contract: FiniteForContractKind,
    pub len_member: Option<MemberId>,
    pub index_member: Option<MemberId>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCheck {
    pub expr: ExprId,
    pub result: Shape,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteForContractKind {
    Sequence,
    Range,
    MemberAccess,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializerCheck {
    pub target: Shape,
    pub materializer: Option<MemberId>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallTarget {
    TopLevel(tune_hir::HirId),
    Member(MemberId),
    Variant(VariantId),
    Bound,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSignature {
    pub target: CallTarget,
    pub params: Vec<Shape>,
    pub ret: Shape,
    pub receiver: Option<Shape>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallCheck {
    pub expr: ExprId,
    pub target: CallTarget,
    pub args: Vec<Shape>,
    pub params: Vec<Shape>,
    pub ret: Shape,
    pub receiver: Option<Shape>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnCheck {
    pub expr: ExprId,
    pub shape: Shape,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeAnalysis {
    pub inferred_signature: Option<CallSignature>,
    pub frame: StateFrame,
    pub expr_shapes: Vec<ExprShape>,
    pub calls: Vec<CallCheck>,
    pub returns: Vec<ReturnCheck>,
    pub assignments: Vec<AssignmentCheck>,
    pub finite_for: Vec<FiniteForCheck>,
    pub spawn: Vec<SpawnCheck>,
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
        calls: Vec::new(),
        returns: Vec::new(),
        assignments: Vec::new(),
        finite_for: Vec::new(),
        spawn: Vec::new(),
        materializers: Vec::new(),
        diagnostics: Vec::new(),
        inferred_signature: None,
        expected_stack: Vec::new(),
    };
    analyzer.seed_item(item);
    analyzer.check_public_api_shape(item);
    if let Some(body) = &item.body {
        let actual = analyzer.analyze_expr(body);
        analyzer.infer_item_signature(item, &actual);
        if let Some(shape) = &item.shape {
            let lowered = lower_resolved_hir_shape(shape, &resolved.scope);
            let expected = lowered.shape;
            analyzer.diagnostics.extend(lowered.diagnostics);
            analyzer.check_result_propagation(item, body, &expected);
            analyzer.check_returns_against(&expected);
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
    calls: Vec<CallCheck>,
    returns: Vec<ReturnCheck>,
    assignments: Vec<AssignmentCheck>,
    finite_for: Vec<FiniteForCheck>,
    spawn: Vec<SpawnCheck>,
    materializers: Vec<MaterializerCheck>,
    diagnostics: Vec<Diagnostic>,
    inferred_signature: Option<CallSignature>,
    expected_stack: Vec<Shape>,
}

impl Analyzer<'_> {
    fn finish(self) -> ShapeAnalysis {
        ShapeAnalysis {
            inferred_signature: self.inferred_signature,
            frame: self.frame,
            expr_shapes: self.expr_shapes,
            calls: self.calls,
            returns: self.returns,
            assignments: self.assignments,
            finite_for: self.finite_for,
            spawn: self.spawn,
            materializers: self.materializers,
            diagnostics: self.diagnostics,
        }
    }

    fn infer_item_signature(&mut self, item: &Item, actual: &Shape) {
        if item.kind != ItemKind::CallableDecl {
            return;
        }
        let ret = item
            .shape
            .as_ref()
            .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope))
            .map_or_else(
                || {
                    Shape::join_all(
                        [actual.clone()]
                            .into_iter()
                            .chain(self.returns.iter().map(|returned| returned.shape.clone())),
                    )
                },
                |lowered| {
                    self.diagnostics.extend(lowered.diagnostics);
                    lowered.shape
                },
            );
        let params = item
            .params
            .iter()
            .map(|param| {
                self.frame
                    .get(BindingKey::Param(param.id))
                    .map_or(Shape::Hole, |binding| binding.storage_shape.clone())
            })
            .collect();
        self.inferred_signature = Some(CallSignature {
            target: CallTarget::TopLevel(item.id),
            params,
            ret,
            receiver: None,
            span: item.span,
        });
    }

    fn seed_item(&mut self, item: &Item) {
        for param in &item.params {
            let shape = self.lower_item_shape_or_hole(item, param.shape.as_ref());
            self.frame.define(BindingState::new(
                BindingKey::Param(param.id),
                param.name.clone(),
                shape.clone(),
                shape,
                param.span,
            ));
        }
    }

    pub(super) fn lower_item_shape_or_hole(
        &mut self,
        item: &Item,
        shape: Option<&ShapeExpr>,
    ) -> Shape {
        let Some(shape) = shape else {
            return Shape::Hole;
        };
        if let ShapeExprKind::Named(name) = &shape.kind
            && let Some(type_param) = item
                .type_params
                .iter()
                .find(|param| param.name.as_deref() == Some(name.as_str()))
        {
            return type_param
                .constraint
                .as_ref()
                .map(|constraint| self.lower_structural_shape(constraint))
                .unwrap_or_else(|| Shape::Param(name.clone()));
        }
        let lowered = lower_resolved_hir_shape(shape, &self.resolved.scope);
        self.diagnostics.extend(lowered.diagnostics);
        lowered.shape
    }

    fn analyze_expr(&mut self, expr: &Expr) -> Shape {
        let shape = match &expr.kind {
            ExprKind::Missing => Shape::Hole,
            ExprKind::Literal(LiteralKind::String(literal)) => {
                let mut has_interpolation = false;
                for part in &literal.parts {
                    if let StringPart::Interpolation(expr) = part {
                        has_interpolation = true;
                        self.analyze_expr(expr);
                    }
                }
                if has_interpolation {
                    Shape::String
                } else {
                    self.literal_or_sequence_shape(expr)
                }
            }
            ExprKind::Literal(_) | ExprKind::Sequence(_) | ExprKind::Tuple(_) => {
                self.literal_or_sequence_shape(expr)
            }
            ExprKind::Struct { name, fields } => {
                for field in fields {
                    let expected = self.struct_field_shape(name, &field.name);
                    let actual = self.analyze_expr_expected(&field.value, &expected);
                    self.constrain_expr_to_shape(&field.value, &expected);
                    self.check_value_against(&expected, &actual, field.value.span);
                }
                self.struct_literal_shape(name)
            }
            ExprKind::Name(_) => self.name_shape(expr),
            ExprKind::Call { callee, args } => self.analyze_call(expr, callee, args),
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
            ExprKind::Return(inner) => {
                let returned = inner
                    .as_deref()
                    .map(|inner| self.analyze_expr(inner))
                    .unwrap_or(Shape::Unit);
                self.returns.push(ReturnCheck {
                    expr: expr.id,
                    shape: returned,
                    span: expr.span,
                });
                Shape::Never
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.analyze_expr(arg);
                }
                Shape::Never
            }
            ExprKind::Spawn(inner) => {
                let result = self.analyze_expr(inner);
                self.spawn.push(SpawnCheck {
                    expr: expr.id,
                    result: result.clone(),
                    span: expr.span,
                });
                Shape::Task(Box::new(result))
            }
            ExprKind::Field { base, .. } => self.field_shape(base, expr),
            ExprKind::Index { base, index } => {
                self.analyze_expr(base);
                self.analyze_expr(index);
                Shape::Hole
            }
            ExprKind::CallableValue { params, body } => self.analyze_callable_value(params, body),
            ExprKind::Loop(body) => self.analyze_loop(body),
            ExprKind::While { condition, body } => self.analyze_while(condition, body),
            ExprKind::Unary { op, expr } => self.analyze_unary(*op, expr),
            ExprKind::Binary { op, lhs, rhs } => self.analyze_binary(*op, expr, lhs, rhs),
            ExprKind::Break | ExprKind::Continue => Shape::Never,
        };
        self.record_expr_shape(expr.id, shape.clone());
        shape
    }

    pub(super) fn analyze_expr_expected(&mut self, expr: &Expr, expected: &Shape) -> Shape {
        if matches!(expected, Shape::Hole) {
            return self.analyze_expr(expr);
        }
        self.expected_stack.push(expected.clone());
        let shape = self.analyze_expr(expr);
        self.expected_stack.pop();
        shape
    }

    pub(super) fn expected_shape(&self) -> Option<&Shape> {
        self.expected_stack
            .iter()
            .rev()
            .find(|shape| !matches!(shape, Shape::Hole))
    }

    fn literal_or_sequence_shape(&mut self, expr: &Expr) -> Shape {
        if let ExprKind::Sequence(elements) = &expr.kind {
            for element in elements {
                self.analyze_expr(element);
            }
        }
        if let ExprKind::Tuple(items) = &expr.kind {
            return Shape::Tuple(items.iter().map(|item| self.analyze_expr(item)).collect());
        }
        let Some(literal) = expr_literal_fact(expr) else {
            return Shape::Hole;
        };
        if literal.is_numeric()
            && let Some(expected) = self.expected_shape()
            && expected.accepts(&Shape::Literal(literal.clone()))
        {
            return expected.clone();
        }
        Shape::Literal(literal)
    }

    fn analyze_unary(&mut self, op: UnaryOp, expr: &Expr) -> Shape {
        let shape = self.analyze_expr(expr);
        match op {
            UnaryOp::Not => Shape::Bool,
            UnaryOp::Neg | UnaryOp::BitNot if Shape::Int.accepts(&shape) => Shape::Int,
            UnaryOp::BitNot if Shape::Byte.accepts(&shape) => Shape::Byte,
            UnaryOp::Neg | UnaryOp::BitNot => Shape::Hole,
        }
    }

    fn struct_literal_shape(&self, name: &str) -> Shape {
        match self.resolved.scope.get(name) {
            Some(binding) if binding.kind == BindingKind::Struct => {
                Shape::Struct(crate::NominalShape::new(binding.id, name))
            }
            _ => Shape::Hole,
        }
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
            .map(|value| self.analyze_expr_expected(value, &expected))
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
                    let storage_shape = if expected == Shape::Hole {
                        if literal.is_numeric() {
                            Shape::Hole
                        } else {
                            literal.storage_shape()
                        }
                    } else {
                        expected.clone()
                    };
                    let binding = BindingState::literal(
                        BindingKey::Local(local),
                        self.local_name(local),
                        storage_shape.clone(),
                        literal,
                        expr.span,
                    );
                    if expected == Shape::Hole {
                        binding
                    } else {
                        binding.with_committed_current(storage_shape)
                    }
                },
            );
            self.frame.define(binding);
        }

        actual
    }

    fn analyze_assign(&mut self, target: &Expr, value: &Expr) -> Shape {
        let actual = self.analyze_expr(value);
        if let Some(key) = self.binding_key(target) {
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
        Shape::Int
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

    fn constrain_expr_to_shape(&mut self, expr: &Expr, expected: &Shape) {
        if matches!(expected, Shape::Hole) {
            return;
        }
        let Some(key) = self.binding_key(expr) else {
            return;
        };
        let Some(binding) = self.frame.get_mut(key) else {
            return;
        };
        if binding.storage_shape == Shape::Hole {
            binding.storage_shape = expected.clone();
        }
        if binding.current_shape == Shape::Hole {
            binding.current_shape = expected.clone();
        }
    }
}
