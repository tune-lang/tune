use tune_hir::expr::{BinaryOp, Expr, ExprKind, LiteralKind, StringPart};
use tune_shape::{MaterializationPlan, Shape};

use super::LowerContext;
use super::values::{default_value_ops, expr_produces_value, if_produces_value};
use crate::plan::{FiniteForContract, PlanIfBranch, PlanMatchArm, PlanOp, StructEscapeReason};

impl LowerContext<'_> {
    pub(super) fn lower_expr(&self, expr: &Expr, ops: &mut Vec<PlanOp>) {
        match &expr.kind {
            ExprKind::Missing => {}
            ExprKind::Name(_) => {
                ops.push(PlanOp::BindingGet {
                    source: self.name_target(expr.id),
                });
            }
            ExprKind::Literal(LiteralKind::Int(text)) => {
                if !self.lower_materialized_numeric_expr(expr, ops) {
                    self.lower_numeric_literal(text, None, ops);
                }
            }
            ExprKind::Literal(LiteralKind::Float(text)) => {
                if !self.lower_materialized_numeric_expr(expr, ops) {
                    self.lower_numeric_literal(text, Some(&Shape::Float), ops);
                }
            }
            ExprKind::Literal(LiteralKind::Bool(value)) => {
                ops.push(PlanOp::ConstBool { value: *value });
            }
            ExprKind::Literal(LiteralKind::None) => {
                ops.push(PlanOp::ConstNone);
            }
            ExprKind::Literal(LiteralKind::String(value)) => {
                if let Some(value) = value.plain_text() {
                    ops.push(PlanOp::ConstString { value });
                } else {
                    let mut part_count = 0;
                    for part in &value.parts {
                        match part {
                            StringPart::Text(value) => {
                                ops.push(PlanOp::ConstString {
                                    value: value.clone(),
                                });
                                part_count += 1;
                            }
                            StringPart::Interpolation(expr) => {
                                self.lower_expr(expr, ops);
                                part_count += 1;
                            }
                        }
                    }
                    ops.push(PlanOp::StringBuild { part_count });
                }
            }
            ExprKind::CallableValue { params: _, body } => {
                ops.push(PlanOp::CallableValue {
                    callable: expr.id,
                    captures: self.callable_value_captures(body),
                    span: expr.span,
                });
            }
            ExprKind::Sequence(elements) => {
                let element_shape = sequence_element_shape(self.expr_shape(expr));
                ops.push(PlanOp::SequenceBuild {
                    element_count: elements.len(),
                    element_shape,
                });
                for element in elements {
                    self.lower_expr(element, ops);
                    ops.push(PlanOp::SequencePush);
                }
            }
            ExprKind::Tuple(elements) => {
                for element in elements {
                    self.lower_expr(element, ops);
                }
                ops.push(PlanOp::TupleBuild {
                    element_count: elements.len(),
                });
            }
            ExprKind::Struct { name, fields } => {
                let ordered = self.struct_field_inits(name, fields);
                for (field, value) in &ordered {
                    self.lower_expr_for_shape(value, self.struct_field_shape(name, *field), ops);
                }
                if let Some(item) = self.struct_item_id(name) {
                    ops.push(PlanOp::StructConstruct {
                        item,
                        escape: self.struct_escape,
                        state: crate::StructStatePlan::for_escape(self.struct_escape),
                        fields: ordered.into_iter().map(|(field, _)| field).collect(),
                        span: expr.span,
                    });
                }
            }
            ExprKind::Call { callee, args } => {
                self.lower_call(expr.id, callee, args, ops);
            }
            ExprKind::Field { base, name } => {
                if self.name_target(expr.id).is_some() {
                    ops.push(PlanOp::BindingGet {
                        source: self.name_target(expr.id),
                    });
                    return;
                }
                self.lower_expr(base, ops);
                let field = name.clone().unwrap_or_default();
                ops.push(PlanOp::FieldGet {
                    member: self.field_member(base, &field),
                    field,
                    span: expr.span,
                });
            }
            ExprKind::Index { base, index } => {
                self.lower_expr(base, ops);
                if matches!(self.expr_shape(base), Some(Shape::String)) {
                    self.lower_expr_for_shape(index, Some(Shape::Size), ops);
                    ops.push(PlanOp::StringGet { span: expr.span });
                } else {
                    self.lower_expr(index, ops);
                    ops.push(PlanOp::SequenceGet {
                        checked: true,
                        index_member: self.index_member(base),
                    });
                }
            }
            ExprKind::Let { shape, value, .. } => {
                let mut initialized = false;
                if let Some(value) = value {
                    initialized = true;
                    let context = if self
                        .local_for_expr(expr.id)
                        .is_some_and(|local| self.captured_locals.contains(&local))
                    {
                        self.with_struct_escape(StructEscapeReason::Captured)
                    } else {
                        self.clone_context()
                    };
                    context.lower_expr_for_binding(value, shape.as_ref(), ops);
                    if matches!(value.kind, ExprKind::Sequence(_))
                        && let Some(target) = context.lower_shape(shape.as_ref())
                    {
                        let materializer = context.sequence_materializer(&target);
                        ops.push(PlanOp::Materialize {
                            plan: MaterializationPlan {
                                target: target.clone(),
                                commitment: tune_shape::Commitment::CommitBinding,
                            },
                            materializer,
                        });
                    }
                } else if let Some(default_ops) = self
                    .lower_shape(shape.as_ref())
                    .and_then(|shape| default_value_ops(&shape))
                    .filter(|ops| !ops.is_empty())
                {
                    initialized = true;
                    ops.extend(default_ops);
                }
                ops.push(PlanOp::LocalLet {
                    local: self.local_for_expr(expr.id),
                    initialized,
                });
            }
            ExprKind::Assign { target, value } => {
                self.lower_assignment(target, value, ops);
            }
            ExprKind::Unary { op, expr: inner } => {
                let shape = self.expr_shape(expr).unwrap_or(Shape::Hole);
                self.lower_expr_for_shape(inner, Some(shape.clone()), ops);
                ops.push(PlanOp::UnaryOp { op: *op, shape });
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let expr_shape = self.expr_shape(expr).unwrap_or(Shape::Hole);
                if is_bool_and_or(*op, &expr_shape) {
                    let lhs_ops = self.lower_expr_to_ops(lhs);
                    let rhs_ops = self.lower_expr_to_ops(rhs);
                    if matches!(op, BinaryOp::And | BinaryOp::BitAnd) {
                        ops.push(PlanOp::BoolAnd {
                            lhs_ops,
                            rhs_ops,
                            span: expr.span,
                        });
                    } else {
                        ops.push(PlanOp::BoolOr {
                            lhs_ops,
                            rhs_ops,
                            span: expr.span,
                        });
                    }
                } else if let Some((value, is_not)) = none_check_operand(*op, lhs, rhs) {
                    self.lower_expr(value, ops);
                    ops.push(PlanOp::NoneCheck {
                        is_not,
                        span: expr.span,
                    });
                } else {
                    let shape = if is_comparison_op(*op) {
                        self.expr_shape(lhs).unwrap_or(Shape::Hole)
                    } else {
                        expr_shape
                    };
                    if is_contextual_numeric_op(*op)
                        || is_contextual_bit_op(*op)
                        || is_comparison_op(*op)
                    {
                        self.lower_expr_for_shape(lhs, Some(shape.clone()), ops);
                        self.lower_expr_for_shape(rhs, Some(shape.clone()), ops);
                    } else {
                        self.lower_expr(lhs, ops);
                        self.lower_expr(rhs, ops);
                    }
                    ops.push(PlanOp::BinaryOp {
                        op: *op,
                        shape,
                        span: expr.span,
                    });
                }
            }
            ExprKind::Spawn(inner) => {
                let mut body_ops = Vec::new();
                self.with_struct_escape(StructEscapeReason::SpawnBoundary)
                    .lower_expr(inner, &mut body_ops);
                ops.push(PlanOp::Spawn {
                    body: inner.id,
                    body_ops,
                    captures: self.callable_value_captures(inner),
                    span: expr.span,
                });
            }
            ExprKind::Propagate(inner) => {
                self.lower_expr(inner, ops);
                ops.push(PlanOp::ResultPropagate {
                    expr: expr.id,
                    span: expr.span,
                });
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                let default_else_ops = else_branch
                    .is_none()
                    .then(|| {
                        self.expr_shape(expr)
                            .and_then(|shape| default_value_ops(&shape))
                    })
                    .flatten()
                    .unwrap_or_default();
                let has_default_else = else_branch.is_none() && !default_else_ops.is_empty();
                ops.push(PlanOp::If {
                    branches: branches
                        .iter()
                        .map(|branch| PlanIfBranch {
                            condition: branch.condition.id,
                            body: branch.body.id,
                            condition_ops: self.lower_expr_to_ops(&branch.condition),
                            body_ops: self.lower_expr_to_ops(&branch.body),
                        })
                        .collect(),
                    else_body: else_branch.as_ref().map(|branch| branch.id),
                    else_ops: else_branch
                        .as_ref()
                        .map_or(default_else_ops, |branch| self.lower_expr_to_ops(branch)),
                    produces_value: if_produces_value(
                        branches,
                        else_branch.as_deref(),
                        self.analysis,
                        has_default_else,
                    ),
                    span: expr.span,
                });
            }
            ExprKind::Match { scrutinee, arms } => {
                if self.lower_structural_match(scrutinee, arms, ops) {
                    return;
                }
                self.lower_expr(scrutinee, ops);
                ops.push(PlanOp::Match {
                    scrutinee: scrutinee.id,
                    arms: arms
                        .iter()
                        .map(|arm| PlanMatchArm {
                            pattern: arm.pattern.clone(),
                            body: arm.body.id,
                            variant: self.pattern_variant(&arm.pattern),
                            tests: self.pattern_tests(&arm.pattern),
                            bindings: self.pattern_bindings(&arm.pattern),
                            body_ops: self.lower_expr_to_ops(&arm.body),
                        })
                        .collect(),
                    produces_value: arms.iter().all(|arm| expr_produces_value(&arm.body)),
                    span: expr.span,
                });
            }
            ExprKind::While { condition, body } => {
                ops.push(PlanOp::While {
                    condition: condition.id,
                    body: body.id,
                    condition_ops: self.lower_expr_to_ops(condition),
                    body_ops: self.lower_expr_to_ops(body),
                    span: expr.span,
                });
            }
            ExprKind::Loop(body) => {
                ops.push(PlanOp::Loop {
                    body: body.id,
                    body_ops: self.lower_expr_to_ops(body),
                    span: expr.span,
                });
            }
            ExprKind::Break => ops.push(PlanOp::Break),
            ExprKind::Continue => ops.push(PlanOp::Continue),
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.with_struct_escape(StructEscapeReason::Returned)
                        .lower_expr(inner, ops);
                }
                ops.push(PlanOp::Return);
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.lower_expr(arg, ops);
                }
                ops.push(PlanOp::Panic {
                    arg_count: args.len(),
                    span: expr.span,
                });
            }
            ExprKind::For {
                pattern,
                iterable,
                body,
            } => {
                let contract = self.finite_for_contract(iterable);
                ops.push(PlanOp::FiniteFor {
                    pattern: pattern.clone(),
                    iterable: iterable.id,
                    body: body.id,
                    binding: self.for_pattern_binding(pattern),
                    iterable_ops: self.lower_expr_to_ops(iterable),
                    body_ops: self.lower_expr_to_ops(body),
                    contract,
                    span: expr.span,
                });
            }
            ExprKind::Block(exprs) => {
                for expr in exprs {
                    self.lower_expr(expr, ops);
                }
            }
        }
    }

    fn finite_for_contract(&self, iterable: &Expr) -> FiniteForContract {
        self.analysis
            .and_then(|analysis| {
                analysis
                    .finite_for
                    .iter()
                    .find(|check| check.iterable == iterable.id)
            })
            .map_or_else(
                || FiniteForContract {
                    source: iterable.id,
                    kind: crate::FiniteForContractKind::Unknown,
                    len_member: None,
                    index_member: None,
                    source_evaluated_once: true,
                    length_evaluated_once: true,
                },
                |check| FiniteForContract {
                    source: check.iterable,
                    kind: match check.contract {
                        tune_shape::FiniteForContractKind::Sequence => {
                            crate::FiniteForContractKind::Sequence
                        }
                        tune_shape::FiniteForContractKind::Range => {
                            crate::FiniteForContractKind::Range
                        }
                        tune_shape::FiniteForContractKind::MemberAccess => {
                            crate::FiniteForContractKind::MemberAccess
                        }
                        tune_shape::FiniteForContractKind::Unknown => {
                            crate::FiniteForContractKind::Unknown
                        }
                    },
                    len_member: check.len_member,
                    index_member: check.index_member,
                    source_evaluated_once: true,
                    length_evaluated_once: true,
                },
            )
    }

    pub(super) fn lower_expr_for_binding(
        &self,
        expr: &Expr,
        shape: Option<&tune_hir::shape::ShapeExpr>,
        ops: &mut Vec<PlanOp>,
    ) {
        if self.lower_materialized_numeric_expr(expr, ops) {
            return;
        }
        if let Some(target) = self.lower_shape(shape)
            && self.lower_numeric_expr_for_target(expr, &target, ops)
        {
            return;
        }
        self.lower_expr(expr, ops);
    }

    pub(super) fn lower_expr_for_shape(
        &self,
        expr: &Expr,
        shape: Option<Shape>,
        ops: &mut Vec<PlanOp>,
    ) {
        if self.lower_materialized_numeric_expr(expr, ops) {
            return;
        }
        if let Some(target) = shape.as_ref()
            && self.lower_numeric_expr_for_target(expr, target, ops)
        {
            return;
        }
        self.lower_expr(expr, ops);
    }
}

fn none_check_operand<'a>(op: BinaryOp, lhs: &'a Expr, rhs: &'a Expr) -> Option<(&'a Expr, bool)> {
    match op {
        BinaryOp::Equal if is_none_literal(rhs) => Some((lhs, false)),
        BinaryOp::Equal if is_none_literal(lhs) => Some((rhs, false)),
        BinaryOp::NotEqual if is_none_literal(rhs) => Some((lhs, true)),
        BinaryOp::NotEqual if is_none_literal(lhs) => Some((rhs, true)),
        _ => None,
    }
}

fn is_comparison_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
    )
}

fn is_contextual_numeric_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
    )
}

fn is_contextual_bit_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Or
            | BinaryOp::And
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::BitAnd
            | BinaryOp::ShiftLeft
            | BinaryOp::ShiftRight
    )
}

fn is_bool_and_or(op: BinaryOp, shape: &Shape) -> bool {
    matches!(shape, Shape::Bool)
        && matches!(
            op,
            BinaryOp::And | BinaryOp::Or | BinaryOp::BitAnd | BinaryOp::BitOr
        )
}

fn is_none_literal(expr: &Expr) -> bool {
    matches!(expr.kind, ExprKind::Literal(LiteralKind::None))
}

fn sequence_element_shape(shape: Option<Shape>) -> Shape {
    match shape {
        Some(Shape::Sequence(element)) => *element,
        Some(Shape::Literal(literal)) => match literal.storage_shape() {
            Shape::Sequence(element) => *element,
            _ => Shape::Hole,
        },
        _ => Shape::Hole,
    }
}
