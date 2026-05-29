use tune_hir::expr::{Expr, ExprKind};
mod assign;
mod callables;
mod calls;
mod captures;
mod expr;
mod materialization;
mod members;
mod module;
mod patterns;
mod returns;
mod specialize;
mod structural;
mod values;

use tune_hir::ExprId;
use tune_hir::item::{Item, ItemKind, StructMember};
use tune_hir::module::Module;
use tune_resolve::{LocalId, NameTarget, ResolvedModule};
use tune_shape::MaterializationPlan;

pub use module::{lower_analyzed_module_to_plan, lower_resolved_module_to_plan};

use crate::plan::{PlanFunction, PlanOp, PlanStructLayout, StructEscapeReason};

use self::values::falls_through;

#[must_use]
pub fn lower_to_plan(name: &str) -> PlanFunction {
    PlanFunction {
        owner: None,
        member: None,
        callable: None,
        name: name.into(),
        span: None,
        params: Vec::new(),
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        struct_layouts: Vec::new(),
        ops: Vec::new(),
    }
}

#[must_use]
pub fn lower_item_to_plan(item: &Item) -> Option<PlanFunction> {
    lower_item_with_context(item, None, None, None)
}

#[must_use]
pub fn lower_resolved_item_to_plan(item: &Item, resolved: &ResolvedModule) -> Option<PlanFunction> {
    lower_item_with_context(item, Some(resolved), None, None)
}

#[must_use]
pub fn lower_resolved_module_item_to_plan(
    module: &Module,
    item: &Item,
    resolved: &ResolvedModule,
) -> Option<PlanFunction> {
    lower_item_with_context(item, Some(resolved), Some(module), None)
}

#[must_use]
pub fn lower_analyzed_module_item_to_plan(
    module: &Module,
    item: &Item,
    resolved: &ResolvedModule,
    analysis: &tune_shape::ShapeAnalysis,
) -> Option<PlanFunction> {
    lower_item_with_context(item, Some(resolved), Some(module), Some(analysis))
}

fn lower_item_with_context(
    item: &Item,
    resolved: Option<&ResolvedModule>,
    module: Option<&Module>,
    analysis: Option<&tune_shape::ShapeAnalysis>,
) -> Option<PlanFunction> {
    let body = item.body.as_ref()?;
    let mut plan = PlanFunction {
        owner: Some(item.id),
        member: None,
        callable: None,
        name: item
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_owned()),
        span: item.span,
        params: item.params.iter().map(|param| param.id).collect(),
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        struct_layouts: module.map_or_else(Vec::new, struct_layouts),
        ops: Vec::new(),
    };
    let mut context = LowerContext {
        resolved,
        module,
        analysis,
        self_shape: None,
        struct_escape: StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: Vec::new(),
        captured_locals: Vec::new(),
    };
    if resolved.is_some() {
        context.captured_locals = context.captured_locals_in_callable_values(body);
    }
    if item.kind == tune_hir::item::ItemKind::CallableDecl {
        context.lower_return_expr(body, &mut plan.ops);
    } else {
        context.lower_expr(body, &mut plan.ops);
    }
    if matches!(body.kind, ExprKind::Sequence(_))
        && let Some(target) = context.lower_shape(item.shape.as_ref())
    {
        let materializer = context.sequence_materializer(&target);
        plan.ops.push(PlanOp::Materialize {
            plan: MaterializationPlan {
                target: target.clone(),
                commitment: tune_shape::Commitment::CommitBinding,
            },
            materializer,
        });
    }
    if falls_through(body, analysis) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}

pub(super) fn struct_layouts(module: &Module) -> Vec<PlanStructLayout> {
    module
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::Struct)
        .map(|item| PlanStructLayout {
            owner: item.id,
            fields: item
                .struct_members
                .iter()
                .filter_map(|member| match member {
                    StructMember::Field(field) => Some(field.id),
                    _ => None,
                })
                .collect(),
        })
        .collect()
}

pub(super) struct LowerContext<'a> {
    pub(super) resolved: Option<&'a ResolvedModule>,
    pub(super) module: Option<&'a Module>,
    pub(super) analysis: Option<&'a tune_shape::ShapeAnalysis>,
    pub(super) self_shape: Option<tune_shape::Shape>,
    pub(super) struct_escape: StructEscapeReason,
    pub(super) structural_witnesses: Vec<StructuralWitness>,
    pub(super) param_shapes: Vec<(tune_hir::MemberId, tune_shape::Shape)>,
    pub(super) captured_locals: Vec<LocalId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StructuralWitness {
    pub(super) local: LocalId,
    pub(super) source: NameTarget,
    pub(super) member: tune_hir::MemberId,
    pub(super) name: String,
    pub(super) kind: StructuralWitnessKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StructuralWitnessKind {
    Field,
    Callable,
}

impl LowerContext<'_> {
    pub(super) fn lower_expr_to_ops(&self, expr: &Expr) -> Vec<PlanOp> {
        let mut ops = Vec::new();
        self.lower_expr(expr, &mut ops);
        ops
    }

    pub(super) fn name_target(&self, expr: ExprId) -> Option<NameTarget> {
        self.resolved?
            .name_refs
            .iter()
            .find(|name_ref| name_ref.expr == expr)
            .map(|name_ref| name_ref.target)
    }

    pub(super) fn local_for_expr(&self, expr: ExprId) -> Option<LocalId> {
        self.resolved?
            .locals
            .iter()
            .find(|local| local.expr == Some(expr))
            .map(|local| local.id)
    }

    pub(super) fn local_kind(&self, local: LocalId) -> Option<tune_resolve::LocalKind> {
        self.resolved?
            .locals
            .iter()
            .find(|binding| binding.id == local)
            .map(|binding| binding.kind)
    }

    pub(super) fn top_level_is_value_binding(&self, item: tune_hir::HirId) -> bool {
        self.module
            .and_then(|module| module.items.iter().find(|candidate| candidate.id == item))
            .is_none_or(|item| item.kind == tune_hir::item::ItemKind::Let)
    }
}
