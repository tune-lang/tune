use tune_hir::expr::{Expr, ExprKind, LiteralKind};
mod calls;
mod members;
mod module;
mod patterns;
mod values;

use tune_hir::ExprId;
use tune_hir::item::Item;
use tune_hir::module::Module;
use tune_resolve::{LocalId, NameTarget, ResolvedModule};
use tune_shape::MaterializationPlan;

pub use module::lower_resolved_module_to_plan;

use crate::plan::{
    FiniteForContract, PlanFunction, PlanIfBranch, PlanMatchArm, PlanOp, StructStatePlan,
};

use self::values::{expr_produces_value, falls_through, if_produces_value};

#[must_use]
pub fn lower_to_plan(name: &str) -> PlanFunction {
    PlanFunction {
        owner: None,
        member: None,
        name: name.into(),
        params: Vec::new(),
        module_bindings: Vec::new(),
        ops: Vec::new(),
    }
}

#[must_use]
pub fn lower_item_to_plan(item: &Item) -> Option<PlanFunction> {
    lower_item_with_context(item, None, None)
}

#[must_use]
pub fn lower_resolved_item_to_plan(item: &Item, resolved: &ResolvedModule) -> Option<PlanFunction> {
    lower_item_with_context(item, Some(resolved), None)
}

#[must_use]
pub fn lower_resolved_module_item_to_plan(
    module: &Module,
    item: &Item,
    resolved: &ResolvedModule,
) -> Option<PlanFunction> {
    lower_item_with_context(item, Some(resolved), Some(module))
}

fn lower_item_with_context(
    item: &Item,
    resolved: Option<&ResolvedModule>,
    module: Option<&Module>,
) -> Option<PlanFunction> {
    let body = item.body.as_ref()?;
    let mut plan = PlanFunction {
        owner: Some(item.id),
        member: None,
        name: item
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_owned()),
        params: item.params.iter().map(|param| param.id).collect(),
        module_bindings: Vec::new(),
        ops: Vec::new(),
    };
    let analysis = module
        .zip(resolved)
        .map(|(module, resolved)| tune_shape::analyze_item(module, resolved, item));
    let context = LowerContext {
        resolved,
        module,
        analysis: analysis.as_ref(),
        self_shape: None,
    };
    context.lower_expr(body, &mut plan.ops);
    if matches!(body.kind, ExprKind::Sequence(_))
        && let Some(target) = context.lower_shape(item.shape.as_ref())
    {
        plan.ops.push(PlanOp::Materialize {
            plan: MaterializationPlan {
                target,
                commitment: tune_shape::Commitment::CommitBinding,
            },
        });
    }
    if falls_through(body) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}

pub(super) struct LowerContext<'a> {
    pub(super) resolved: Option<&'a ResolvedModule>,
    pub(super) module: Option<&'a Module>,
    pub(super) analysis: Option<&'a tune_shape::ShapeAnalysis>,
    pub(super) self_shape: Option<tune_shape::Shape>,
}

impl LowerContext<'_> {
    pub(super) fn lower_expr(&self, expr: &Expr, ops: &mut Vec<PlanOp>) {
        match &expr.kind {
            ExprKind::Missing => {}
            ExprKind::Name(_) => ops.push(PlanOp::BindingGet {
                source: self.name_target(expr.id),
            }),
            ExprKind::Literal(LiteralKind::Int(text)) => {
                if let Ok(value) = text.parse::<i64>() {
                    ops.push(PlanOp::ConstInt { value });
                }
            }
            ExprKind::Literal(LiteralKind::Bool(value)) => {
                ops.push(PlanOp::ConstBool { value: *value });
            }
            ExprKind::Literal(_) => {}
            ExprKind::CallableValue { params: _, body } => {
                self.lower_expr(body, ops);
                ops.push(PlanOp::CallableValue);
            }
            ExprKind::Sequence(elements) => {
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
                        state: StructStatePlan::LOCAL,
                        fields: ordered.into_iter().map(|(field, _)| field).collect(),
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
                    self.lower_expr(value, ops);
                    if matches!(value.kind, ExprKind::Sequence(_))
                        && let Some(target) = self.lower_shape(shape.as_ref())
                    {
                        ops.push(PlanOp::Materialize {
                            plan: MaterializationPlan {
                                target,
                                commitment: tune_shape::Commitment::CommitBinding,
                            },
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
                self.lower_expr(lhs, ops);
                self.lower_expr(rhs, ops);
                ops.push(PlanOp::BinaryOp { op: *op });
            }
            ExprKind::Spawn(inner) => {
                self.lower_expr(inner, ops);
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
                self.lower_expr(condition, ops);
                self.lower_expr(body, ops);
                ops.push(PlanOp::While {
                    condition: condition.id,
                    body: body.id,
                    span: expr.span,
                });
            }
            ExprKind::Loop(body) => {
                self.lower_expr(body, ops);
                ops.push(PlanOp::Loop {
                    body: body.id,
                    span: expr.span,
                });
            }
            ExprKind::Break => ops.push(PlanOp::Break),
            ExprKind::Continue => ops.push(PlanOp::Continue),
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.lower_expr(inner, ops);
                }
                ops.push(PlanOp::Return);
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.lower_expr(arg, ops);
                }
                ops.push(PlanOp::Panic);
            }
            ExprKind::For {
                pattern,
                iterable,
                body,
            } => {
                self.lower_expr(iterable, ops);
                self.lower_expr(body, ops);
                ops.push(PlanOp::FiniteFor {
                    pattern: pattern.clone(),
                    iterable: iterable.id,
                    body: body.id,
                    contract: FiniteForContract {
                        source: iterable.id,
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

    fn lower_assignment(&self, target: &Expr, value: &Expr, ops: &mut Vec<PlanOp>) {
        match &target.kind {
            ExprKind::Name(_) => {
                self.lower_expr(value, ops);
                ops.push(PlanOp::BindingSet {
                    target: self.name_target(target.id),
                });
            }
            ExprKind::Field { base, name } => {
                self.lower_expr(base, ops);
                self.lower_expr(value, ops);
                let field = name.clone().unwrap_or_default();
                ops.push(PlanOp::FieldSet {
                    member: self.field_member(base, &field),
                    field,
                    base: self.field_base_target(base),
                });
            }
            ExprKind::Index { base, index } => {
                self.lower_expr(base, ops);
                self.lower_expr(index, ops);
                self.lower_expr(value, ops);
                ops.push(PlanOp::SequenceSet {
                    checked: true,
                    index_member: self.index_member(base),
                });
            }
            _ => {
                self.lower_expr(target, ops);
                self.lower_expr(value, ops);
                ops.push(PlanOp::Assign);
            }
        }
    }

    fn lower_expr_to_ops(&self, expr: &Expr) -> Vec<PlanOp> {
        let mut ops = Vec::new();
        self.lower_expr(expr, &mut ops);
        ops
    }

    fn name_target(&self, expr: ExprId) -> Option<NameTarget> {
        self.resolved?
            .name_refs
            .iter()
            .find(|name_ref| name_ref.expr == expr)
            .map(|name_ref| name_ref.target)
    }

    fn local_for_expr(&self, expr: ExprId) -> Option<LocalId> {
        self.resolved?
            .locals
            .iter()
            .find(|local| local.expr == Some(expr))
            .map(|local| local.id)
    }
}
