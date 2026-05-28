use tune_hir::expr::{BinaryOp, Expr, ExprKind, LiteralKind};
use tune_shape::MaterializationPlan;

use super::LowerContext;
use super::values::{expr_produces_value, if_produces_value};
use crate::plan::{FiniteForContract, PlanIfBranch, PlanMatchArm, PlanOp, StructEscapeReason};

impl LowerContext<'_> {
    pub(super) fn lower_expr(&self, expr: &Expr, ops: &mut Vec<PlanOp>) {
        match &expr.kind {
            ExprKind::Missing => {}
            ExprKind::Name(_) => {
                if self.lower_structural_witness_get(expr, ops) {
                    return;
                }
                ops.push(PlanOp::BindingGet {
                    source: self.name_target(expr.id),
                });
            }
            ExprKind::Literal(LiteralKind::Int(text)) => {
                if let Ok(value) = text.parse::<i64>() {
                    ops.push(PlanOp::ConstInt { value });
                }
            }
            ExprKind::Literal(LiteralKind::Bool(value)) => {
                ops.push(PlanOp::ConstBool { value: *value });
            }
            ExprKind::Literal(LiteralKind::String(value)) => {
                ops.push(PlanOp::ConstString {
                    value: value.clone(),
                });
            }
            ExprKind::Literal(_) => {}
            ExprKind::CallableValue { params: _, body } => {
                self.lower_expr(body, ops);
                ops.push(PlanOp::CallableValue {
                    captures: self.callable_value_captures(body),
                });
            }
            ExprKind::Sequence(elements) => {
                ops.push(PlanOp::SequenceBuild {
                    element_count: elements.len(),
                });
                for element in elements {
                    self.lower_expr(element, ops);
                    ops.push(PlanOp::SequencePush);
                }
            }
            ExprKind::Struct { name, fields } => {
                let ordered = self.struct_field_inits(name, fields);
                for (_, value) in &ordered {
                    self.lower_expr(value, ops);
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
                self.lower_call(callee, args, ops);
            }
            ExprKind::Field { base, name } => {
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
                self.lower_expr(index, ops);
                ops.push(PlanOp::SequenceGet {
                    checked: true,
                    index_member: self.index_member(base),
                });
            }
            ExprKind::Let { shape, value, .. } => {
                let initialized = value.is_some();
                if let Some(value) = value {
                    let context = if self
                        .local_for_expr(expr.id)
                        .is_some_and(|local| self.captured_locals.contains(&local))
                    {
                        self.with_struct_escape(StructEscapeReason::Captured)
                    } else {
                        self.clone_context()
                    };
                    context.lower_expr(value, ops);
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
                }
                ops.push(PlanOp::LocalLet {
                    local: self.local_for_expr(expr.id),
                    initialized,
                });
            }
            ExprKind::Assign { target, value } => {
                self.lower_assignment(target, value, ops);
            }
            ExprKind::Unary { op, expr } => {
                self.lower_expr(expr, ops);
                ops.push(PlanOp::UnaryOp { op: *op });
            }
            ExprKind::Binary { op, lhs, rhs } => {
                if matches!(op, BinaryOp::And | BinaryOp::Or) {
                    let lhs_ops = self.lower_expr_to_ops(lhs);
                    let rhs_ops = self.lower_expr_to_ops(rhs);
                    if matches!(op, BinaryOp::And) {
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
                } else {
                    self.lower_expr(lhs, ops);
                    self.lower_expr(rhs, ops);
                    ops.push(PlanOp::BinaryOp {
                        op: *op,
                        span: expr.span,
                    });
                }
            }
            ExprKind::Spawn(inner) => {
                self.with_struct_escape(StructEscapeReason::SpawnBoundary)
                    .lower_expr(inner, ops);
                ops.push(PlanOp::Spawn {
                    body: inner.id,
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
                        .map_or_else(Vec::new, |branch| self.lower_expr_to_ops(branch)),
                    produces_value: if_produces_value(branches, else_branch.as_deref()),
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
                });
            }
            ExprKind::For {
                pattern,
                iterable,
                body,
            } => {
                ops.push(PlanOp::FiniteFor {
                    pattern: pattern.clone(),
                    iterable: iterable.id,
                    body: body.id,
                    binding: self.for_pattern_binding(pattern),
                    iterable_ops: self.lower_expr_to_ops(iterable),
                    body_ops: self.lower_expr_to_ops(body),
                    contract: FiniteForContract {
                        source: iterable.id,
                        kind: self.finite_for_contract_kind(iterable),
                        len_member: self.len_member(iterable),
                        index_member: self.index_member(iterable),
                        source_evaluated_once: true,
                        length_evaluated_once: true,
                    },
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
}
