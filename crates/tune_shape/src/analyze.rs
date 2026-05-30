use std::collections::HashMap;

use tune_diagnostics::{Diagnostic, Span};
use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart};
mod bindings;
mod callable;
mod calls;
mod contracts;
mod control;
mod diagnostics;
mod fields;
mod generics;
mod item_shapes;
mod operators;
mod values;

use tune_hir::item::{Item, ItemKind, Param};
use tune_hir::module::Module;
use tune_hir::shape::{ShapeExpr, ShapeExprKind};
use tune_hir::{ExprId, MemberId};
use tune_resolve::{ResolvedModule, VariantId};

use crate::{
    BindingKey, BindingState, ExprMaterialization, Shape, StateFrame, lower_resolved_hir_shape,
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
    StringLen,
    TaskJoin,
    Bound,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSignature {
    pub target: CallTarget,
    pub params: Vec<Shape>,
    pub param_type_params: Vec<Option<String>>,
    pub ret: Shape,
    pub type_params: Vec<String>,
    pub type_args: Vec<Shape>,
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
    pub type_args: Vec<Shape>,
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
    pub item_current_shape: Shape,
    pub inferred_signature: Option<CallSignature>,
    pub frame: StateFrame,
    pub expr_shapes: Vec<ExprShape>,
    pub calls: Vec<CallCheck>,
    pub returns: Vec<ReturnCheck>,
    pub assignments: Vec<AssignmentCheck>,
    pub finite_for: Vec<FiniteForCheck>,
    pub spawn: Vec<SpawnCheck>,
    pub materializers: Vec<MaterializerCheck>,
    pub materializations: Vec<ExprMaterialization>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn analyze_item(module: &Module, resolved: &ResolvedModule, item: &Item) -> ShapeAnalysis {
    analyze_item_with_top_level_shapes(module, resolved, item, &HashMap::new())
}

fn analyze_item_with_top_level_shapes(
    module: &Module,
    resolved: &ResolvedModule,
    item: &Item,
    top_level_shapes: &HashMap<tune_hir::HirId, Shape>,
) -> ShapeAnalysis {
    let mut analyzer = Analyzer {
        module,
        resolved,
        top_level_shapes,
        item_current_shape: declared_item_current_shape(item, resolved),
        frame: StateFrame::new(),
        expr_shapes: Vec::new(),
        calls: Vec::new(),
        returns: Vec::new(),
        assignments: Vec::new(),
        finite_for: Vec::new(),
        spawn: Vec::new(),
        materializers: Vec::new(),
        materializations: Vec::new(),
        diagnostics: Vec::new(),
        inferred_signature: None,
        expected_stack: Vec::new(),
    };
    analyzer.seed_item(item);
    analyzer.check_public_api_shape(item);
    if let Some(body) = &item.body {
        if let Some(shape) = &item.shape {
            let expected = analyzer.lower_item_shape_or_hole(item, Some(shape));
            let actual = analyzer.analyze_expr_expected(body, &expected);
            analyzer.infer_item_signature(item, &actual);
            analyzer.commit_item_current_shape(item, &expected, &actual);
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
            let actual = analyzer.analyze_expr(body);
            analyzer.infer_item_signature(item, &actual);
            analyzer.commit_item_current_shape(item, &Shape::Hole, &actual);
            analyzer.check_untyped_result_propagation(item, body);
            if item.kind == ItemKind::Let {
                analyzer.check_unannotated_optional_copy(&actual, body.span);
            }
        }
    }
    analyzer.finish()
}

#[must_use]
pub fn analyze_module(module: &Module, resolved: &ResolvedModule) -> Vec<ShapeAnalysis> {
    let mut top_level_shapes = HashMap::new();
    let mut analyses = Vec::new();
    for item in &module.items {
        let analysis =
            analyze_item_with_top_level_shapes(module, resolved, item, &top_level_shapes);
        top_level_shapes.insert(item.id, analysis.item_current_shape.clone());
        analyses.push(analysis);
    }
    analyses
}

struct Analyzer<'a> {
    module: &'a Module,
    resolved: &'a ResolvedModule,
    top_level_shapes: &'a HashMap<tune_hir::HirId, Shape>,
    item_current_shape: Shape,
    frame: StateFrame,
    expr_shapes: Vec<ExprShape>,
    calls: Vec<CallCheck>,
    returns: Vec<ReturnCheck>,
    assignments: Vec<AssignmentCheck>,
    finite_for: Vec<FiniteForCheck>,
    spawn: Vec<SpawnCheck>,
    materializers: Vec<MaterializerCheck>,
    materializations: Vec<ExprMaterialization>,
    diagnostics: Vec<Diagnostic>,
    inferred_signature: Option<CallSignature>,
    expected_stack: Vec<Shape>,
}

impl Analyzer<'_> {
    fn finish(self) -> ShapeAnalysis {
        ShapeAnalysis {
            item_current_shape: self.item_current_shape,
            inferred_signature: self.inferred_signature,
            frame: self.frame,
            expr_shapes: self.expr_shapes,
            calls: self.calls,
            returns: self.returns,
            assignments: self.assignments,
            finite_for: self.finite_for,
            spawn: self.spawn,
            materializers: self.materializers,
            materializations: self.materializations,
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
            .map(|shape| self.lower_item_shape_or_hole(item, Some(shape)))
            .unwrap_or_else(|| {
                Shape::join_all(
                    [actual.clone()]
                        .into_iter()
                        .chain(self.returns.iter().map(|returned| returned.shape.clone())),
                )
            });
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
            param_type_params: Vec::new(),
            ret,
            type_params: item
                .type_params
                .iter()
                .filter_map(|param| param.name.clone())
                .collect(),
            type_args: Vec::new(),
            receiver: None,
            span: item.span,
        });
    }

    fn seed_self_value(&mut self, shape: Shape, span: Option<Span>) {
        self.frame.define(BindingState::new(
            BindingKey::SelfValue,
            Some("self".to_owned()),
            shape.clone(),
            shape,
            span,
        ));
    }

    fn seed_member_param(&mut self, param: &Param, owner: &Item) {
        let shape = self.lower_item_shape_or_hole(owner, param.shape.as_ref());
        self.frame.define(BindingState::new(
            BindingKey::Param(param.id),
            param.name.clone(),
            shape.clone(),
            shape,
            param.span,
        ));
    }

    fn seed_member_param_shape(
        &mut self,
        id: MemberId,
        name: Option<String>,
        shape: Shape,
        span: Option<Span>,
    ) {
        self.frame.define(BindingState::new(
            BindingKey::Param(id),
            name,
            shape.clone(),
            shape,
            span,
        ));
    }

    fn commit_item_current_shape(&mut self, item: &Item, declared: &Shape, actual: &Shape) {
        if item.kind == ItemKind::CallableDecl
            && let Some(signature) = &self.inferred_signature
        {
            self.item_current_shape = Shape::Callable {
                params: signature.params.clone(),
                ret: Box::new(signature.ret.clone()),
            };
            return;
        }
        self.item_current_shape = top_level_current_shape_from_actual(declared, actual);
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
        let lowered = item_shapes::lower_item_shape_expr(shape, item, &self.resolved.scope);
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
            ExprKind::Struct { name, fields } => self.analyze_struct_literal(name, fields),
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
                let base_shape = self.analyze_expr(base);
                if base_shape == Shape::String {
                    let index_shape = self.analyze_expr_expected(index, &Shape::Size);
                    self.constrain_expr_to_shape(index, &Shape::Size);
                    self.check_value_against(&Shape::Size, &index_shape, index.span);
                    Shape::String
                } else if let Shape::Sequence(item) = base_shape {
                    let index_shape = self.analyze_expr_expected(index, &Shape::Size);
                    self.constrain_expr_to_shape(index, &Shape::Size);
                    self.check_value_against(&Shape::Size, &index_shape, index.span);
                    item.as_ref().clone()
                } else {
                    self.analyze_expr(index);
                    Shape::Hole
                }
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
        let scoped_shape = match &expr.kind {
            ExprKind::Block(exprs) => {
                let Some((last, prefix)) = exprs.split_last() else {
                    return Shape::Unit;
                };
                for expr in prefix {
                    self.analyze_expr(expr);
                }
                Some(self.analyze_expr_expected(last, expected))
            }
            ExprKind::If {
                branches,
                else_branch,
            } => Some(self.analyze_if_expected(branches, else_branch.as_deref(), expected)),
            ExprKind::Match { scrutinee, arms } => {
                Some(self.analyze_match_expected(expr, scrutinee, arms, expected))
            }
            _ => None,
        };
        if let Some(shape) = scoped_shape {
            self.record_expr_shape(expr.id, shape.clone());
            return shape;
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

fn declared_item_current_shape(item: &Item, resolved: &ResolvedModule) -> Shape {
    match item.kind {
        ItemKind::CallableDecl => Shape::Callable {
            params: item
                .params
                .iter()
                .map(|param| {
                    param
                        .shape
                        .as_ref()
                        .map(|shape| lower_resolved_hir_shape(shape, &resolved.scope).shape)
                        .unwrap_or(Shape::Hole)
                })
                .collect(),
            ret: Box::new(
                item.shape
                    .as_ref()
                    .map(|shape| lower_resolved_hir_shape(shape, &resolved.scope).shape)
                    .unwrap_or(Shape::Hole),
            ),
        },
        _ => item
            .shape
            .as_ref()
            .map(|shape| lower_resolved_hir_shape(shape, &resolved.scope).shape)
            .unwrap_or(Shape::Hole),
    }
}

fn top_level_current_shape_from_actual(storage: &Shape, actual: &Shape) -> Shape {
    match (storage, actual) {
        (Shape::Hole, actual) => actual.clone(),
        (Shape::Optional(_), Shape::Literal(crate::LiteralFact::None)) => {
            Shape::Literal(crate::LiteralFact::None)
        }
        (Shape::Optional(inner), actual) if inner.accepts(actual) => actual.clone(),
        (storage, _) => storage.clone(),
    }
}
