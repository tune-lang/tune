use tune_hir::expr::{Expr, ExprKind, LiteralKind};
mod members;

use tune_hir::ExprId;
use tune_hir::item::Item;
use tune_hir::module::Module;
use tune_resolve::{LocalId, NameTarget, ResolvedModule};
use tune_shape::MaterializationPlan;

use crate::plan::{FiniteForContract, PlanFunction, PlanIfBranch, PlanMatchArm, PlanOp};

#[must_use]
pub fn lower_to_plan(name: &str) -> PlanFunction {
    PlanFunction {
        name: name.into(),
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
        name: item
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_owned()),
        ops: Vec::new(),
    };
    let analysis = module
        .zip(resolved)
        .map(|(module, resolved)| tune_shape::analyze_item(module, resolved, item));
    let context = LowerContext {
        resolved,
        module,
        analysis: analysis.as_ref(),
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

fn falls_through(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Return(_) | ExprKind::Panic(_) | ExprKind::Break | ExprKind::Continue => false,
        ExprKind::Block(exprs) => exprs.last().is_none_or(falls_through),
        ExprKind::If {
            branches,
            else_branch: Some(else_branch),
        } => {
            branches.iter().any(|branch| falls_through(&branch.body)) || falls_through(else_branch)
        }
        ExprKind::Loop(body) => falls_through(body),
        _ => true,
    }
}

struct LowerContext<'a> {
    resolved: Option<&'a ResolvedModule>,
    module: Option<&'a Module>,
    analysis: Option<&'a tune_shape::ShapeAnalysis>,
}

impl LowerContext<'_> {
    fn lower_expr(&self, expr: &Expr, ops: &mut Vec<PlanOp>) {
        match &expr.kind {
            ExprKind::Missing | ExprKind::Name(_) => {}
            ExprKind::Literal(LiteralKind::Int(text)) => {
                if let Ok(value) = text.parse::<i64>() {
                    ops.push(PlanOp::ConstInt { value });
                }
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
            ExprKind::Call { callee, args } => {
                if let Some(base) = task_join_base(callee, args) {
                    self.lower_expr(base, ops);
                    ops.push(PlanOp::TaskJoin);
                    return;
                }

                self.lower_expr(callee, ops);
                for arg in args {
                    self.lower_expr(arg, ops);
                }
                ops.push(self.call_op(callee));
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
                for branch in branches {
                    self.lower_expr(&branch.condition, ops);
                    self.lower_expr(&branch.body, ops);
                }
                if let Some(else_branch) = else_branch {
                    self.lower_expr(else_branch, ops);
                }
                ops.push(PlanOp::If {
                    branches: branches
                        .iter()
                        .map(|branch| PlanIfBranch {
                            condition: branch.condition.id,
                            body: branch.body.id,
                        })
                        .collect(),
                    else_body: else_branch.as_ref().map(|branch| branch.id),
                    span: expr.span,
                });
            }
            ExprKind::Match { scrutinee, arms } => {
                self.lower_expr(scrutinee, ops);
                for arm in arms {
                    self.lower_expr(&arm.body, ops);
                }
                ops.push(PlanOp::Match {
                    scrutinee: scrutinee.id,
                    arms: arms
                        .iter()
                        .map(|arm| PlanMatchArm {
                            pattern: arm.pattern.clone(),
                            body: arm.body.id,
                        })
                        .collect(),
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

    fn call_op(&self, callee: &Expr) -> PlanOp {
        if let ExprKind::Field { base, name } = &callee.kind {
            let name = name.clone().unwrap_or_default();
            return PlanOp::MemberCall {
                member: self.callable_member(base, &name),
                name,
            };
        }

        match self.name_target(callee.id) {
            Some(NameTarget::TopLevel(target)) => PlanOp::DirectCall { target },
            Some(NameTarget::Variant(variant)) => PlanOp::VariantConstruct { variant },
            _ => PlanOp::BoundCall,
        }
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

fn task_join_base<'expr>(callee: &'expr Expr, args: &[Expr]) -> Option<&'expr Expr> {
    if !args.is_empty() {
        return None;
    }

    let ExprKind::Field { base, name } = &callee.kind else {
        return None;
    };

    matches!(name.as_deref(), Some("join")).then_some(base)
}
