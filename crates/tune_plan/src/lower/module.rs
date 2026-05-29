use tune_hir::item::{
    CallableMember, IndexAccess, Item, ItemKind, SequenceMaterializer, StructMember,
};
use tune_hir::module::Module;
use tune_resolve::ResolvedModule;
use tune_shape::{MaterializationPlan, Shape};

use super::LowerContext;
use super::callables::lower_callable_value_functions;
use super::specialize::infer_direct_call_param_shapes_from_analyses;
use crate::plan::{PlanFunction, PlanModule, PlanOp};

#[must_use]
pub fn lower_resolved_module_to_plan(module: &Module, resolved: &ResolvedModule) -> PlanModule {
    let analyses = tune_shape::analyze_module(module, resolved);
    lower_analyzed_module_to_plan(module, resolved, &analyses)
}

#[must_use]
pub fn lower_analyzed_module_to_plan(
    module: &Module,
    resolved: &ResolvedModule,
    analyses: &[tune_shape::ShapeAnalysis],
) -> PlanModule {
    let param_shapes = infer_direct_call_param_shapes_from_analyses(module, resolved, analyses);
    let module_bindings = module
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::Let && item.body.is_some())
        .map(|item| item.id)
        .collect::<Vec<_>>();
    let entry = (!module_bindings.is_empty()).then(|| {
        let last_binding = module_bindings.last().copied();
        let mut entry = PlanFunction {
            owner: None,
            member: None,
            callable: None,
            name: "<entry>".to_owned(),
            span: module.items.first().and_then(|item| item.span),
            params: Vec::new(),
            local_params: Vec::new(),
            captures: Vec::new(),
            module_bindings: module_bindings.clone(),
            ops: Vec::new(),
        };
        for item in module
            .items
            .iter()
            .filter(|item| item.kind == ItemKind::Let && item.body.is_some())
        {
            lower_module_item_into_entry(
                module,
                item,
                resolved,
                analysis_for_item(module, analyses, item),
                last_binding,
                &param_shapes,
                &mut entry.ops,
            );
        }
        entry.ops.push(PlanOp::Return);
        entry
    });
    let functions = module
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::CallableDecl)
        .filter_map(|item| {
            lower_module_callable(
                module,
                resolved,
                item,
                analysis_for_item(module, analyses, item),
                &param_shapes,
            )
        })
        .chain(lower_struct_member_functions(
            module,
            resolved,
            analyses,
            &param_shapes,
        ))
        .chain(lower_callable_value_functions(
            module,
            resolved,
            analyses,
            &param_shapes,
        ))
        .collect();

    PlanModule { entry, functions }
}

fn lower_module_item_into_entry(
    module: &Module,
    item: &Item,
    resolved: &ResolvedModule,
    analysis: Option<&tune_shape::ShapeAnalysis>,
    last_binding: Option<tune_hir::HirId>,
    param_shapes: &[(tune_hir::MemberId, Shape)],
    ops: &mut Vec<PlanOp>,
) {
    let Some(body) = item.body.as_ref() else {
        return;
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis,
        self_shape: None,
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
        captured_locals: captured_locals_for_body(resolved, body),
    };
    context.lower_expr_for_binding(body, item.shape.as_ref(), ops);
    if matches!(body.kind, tune_hir::expr::ExprKind::Sequence(_))
        && let Some(target) = context.lower_shape(item.shape.as_ref())
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
    ops.push(PlanOp::ModuleLet {
        item: item.id,
        initialized: true,
        keep_value: last_binding == Some(item.id),
    });
}

fn lower_module_callable(
    module: &Module,
    resolved: &ResolvedModule,
    item: &Item,
    analysis: Option<&tune_shape::ShapeAnalysis>,
    param_shapes: &[(tune_hir::MemberId, Shape)],
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
        ops: Vec::new(),
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis,
        self_shape: None,
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
        captured_locals: captured_locals_for_body(resolved, body),
    };
    context.lower_return_expr(body, &mut plan.ops);
    if super::falls_through(body) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}

fn lower_struct_member_functions<'a>(
    module: &'a Module,
    resolved: &'a ResolvedModule,
    analyses: &'a [tune_shape::ShapeAnalysis],
    param_shapes: &'a [(tune_hir::MemberId, Shape)],
) -> impl Iterator<Item = PlanFunction> + 'a {
    module
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::Struct)
        .flat_map(move |item| {
            item.struct_members
                .iter()
                .filter_map(move |member| match member {
                    StructMember::Callable(callable) => lower_callable_member(
                        module,
                        resolved,
                        item,
                        callable,
                        analysis_for_item(module, analyses, item),
                        param_shapes,
                    ),
                    StructMember::IndexAccess(access) => lower_index_access_member(
                        module,
                        resolved,
                        item,
                        access,
                        analysis_for_item(module, analyses, item),
                        param_shapes,
                    ),
                    StructMember::SequenceMaterializer(materializer) => {
                        lower_sequence_materializer_member(
                            module,
                            resolved,
                            item,
                            materializer,
                            analysis_for_item(module, analyses, item),
                            param_shapes,
                        )
                    }
                    StructMember::Field(_) => None,
                })
        })
}

fn lower_callable_member(
    module: &Module,
    resolved: &ResolvedModule,
    owner: &Item,
    callable: &CallableMember,
    analysis: Option<&tune_shape::ShapeAnalysis>,
    param_shapes: &[(tune_hir::MemberId, Shape)],
) -> Option<PlanFunction> {
    let body = callable.body.as_ref()?;
    let mut plan = PlanFunction {
        owner: Some(owner.id),
        member: Some(callable.id),
        callable: None,
        name: format!(
            "{}.{}",
            owner.name.as_deref().unwrap_or("<anonymous>"),
            callable.name.as_deref().unwrap_or("<anonymous>")
        ),
        span: callable.span,
        params: std::iter::once(callable.id)
            .chain(callable.params.iter().map(|param| param.id))
            .collect(),
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: Vec::new(),
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis,
        self_shape: owner
            .name
            .as_ref()
            .map(|name| Shape::Struct(tune_shape::NominalShape::new(owner.id, name))),
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
        captured_locals: captured_locals_for_body(resolved, body),
    };
    context.lower_expr_for_binding(body, callable.shape.as_ref(), &mut plan.ops);
    if super::falls_through(body) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}

fn lower_sequence_materializer_member(
    module: &Module,
    resolved: &ResolvedModule,
    owner: &Item,
    materializer: &SequenceMaterializer,
    analysis: Option<&tune_shape::ShapeAnalysis>,
    param_shapes: &[(tune_hir::MemberId, Shape)],
) -> Option<PlanFunction> {
    let body = materializer.body.as_ref()?;
    let mut plan = PlanFunction {
        owner: Some(owner.id),
        member: Some(materializer.id),
        callable: None,
        name: format!("{}.[items]", owner.name.as_deref().unwrap_or("<anonymous>")),
        span: materializer.span,
        params: vec![materializer.id],
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: Vec::new(),
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis,
        self_shape: owner
            .name
            .as_ref()
            .map(|name| Shape::Struct(tune_shape::NominalShape::new(owner.id, name))),
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
        captured_locals: captured_locals_for_body(resolved, body),
    };
    context.lower_return_expr(body, &mut plan.ops);
    if super::falls_through(body) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}

fn lower_index_access_member(
    module: &Module,
    resolved: &ResolvedModule,
    owner: &Item,
    access: &IndexAccess,
    analysis: Option<&tune_shape::ShapeAnalysis>,
    param_shapes: &[(tune_hir::MemberId, Shape)],
) -> Option<PlanFunction> {
    let body = access.body.as_ref()?;
    let mut plan = PlanFunction {
        owner: Some(owner.id),
        member: Some(access.id),
        callable: None,
        name: format!("{}.[index]", owner.name.as_deref().unwrap_or("<anonymous>")),
        span: access.span,
        params: vec![access.id, access.index_param_id],
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: Vec::new(),
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis,
        self_shape: owner
            .name
            .as_ref()
            .map(|name| Shape::Struct(tune_shape::NominalShape::new(owner.id, name))),
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
        captured_locals: captured_locals_for_body(resolved, body),
    };
    context.lower_return_expr(body, &mut plan.ops);
    if super::falls_through(body) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}

fn analysis_for_item<'a>(
    module: &Module,
    analyses: &'a [tune_shape::ShapeAnalysis],
    item: &Item,
) -> Option<&'a tune_shape::ShapeAnalysis> {
    module
        .items
        .iter()
        .position(|candidate| candidate.id == item.id)
        .and_then(|index| analyses.get(index))
}

pub(super) fn captured_locals_for_body(
    resolved: &ResolvedModule,
    body: &tune_hir::expr::Expr,
) -> Vec<tune_resolve::LocalId> {
    let context = LowerContext {
        resolved: Some(resolved),
        module: None,
        analysis: None,
        self_shape: None,
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: Vec::new(),
        captured_locals: Vec::new(),
    };
    context.captured_locals_in_callable_values(body)
}
