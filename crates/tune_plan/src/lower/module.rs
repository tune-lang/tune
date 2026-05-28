use tune_hir::item::{CallableMember, IndexAccess, Item, ItemKind, StructMember};
use tune_hir::module::Module;
use tune_resolve::ResolvedModule;
use tune_shape::{MaterializationPlan, Shape};

use super::LowerContext;
use super::specialize::infer_direct_call_param_shapes;
use crate::plan::{PlanFunction, PlanModule, PlanOp};

#[must_use]
pub fn lower_resolved_module_to_plan(module: &Module, resolved: &ResolvedModule) -> PlanModule {
    let param_shapes = infer_direct_call_param_shapes(module, resolved);
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
            name: "<entry>".to_owned(),
            span: module.items.first().and_then(|item| item.span),
            params: Vec::new(),
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
        .filter_map(|item| lower_module_callable(module, resolved, item, &param_shapes))
        .chain(lower_struct_member_functions(
            module,
            resolved,
            &param_shapes,
        ))
        .collect();

    PlanModule { entry, functions }
}

fn lower_module_item_into_entry(
    module: &Module,
    item: &Item,
    resolved: &ResolvedModule,
    last_binding: Option<tune_hir::HirId>,
    param_shapes: &[(tune_hir::MemberId, Shape)],
    ops: &mut Vec<PlanOp>,
) {
    let Some(body) = item.body.as_ref() else {
        return;
    };
    let analysis = tune_shape::analyze_item(module, resolved, item);
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis: Some(&analysis),
        self_shape: None,
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
    };
    context.lower_expr(body, ops);
    if matches!(body.kind, tune_hir::expr::ExprKind::Sequence(_))
        && let Some(target) = context.lower_shape(item.shape.as_ref())
    {
        ops.push(PlanOp::Materialize {
            plan: MaterializationPlan {
                target,
                commitment: tune_shape::Commitment::CommitBinding,
            },
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
    param_shapes: &[(tune_hir::MemberId, Shape)],
) -> Option<PlanFunction> {
    let body = item.body.as_ref()?;
    let analysis = tune_shape::analyze_item(module, resolved, item);
    let mut plan = PlanFunction {
        owner: Some(item.id),
        member: None,
        name: item
            .name
            .clone()
            .unwrap_or_else(|| "<anonymous>".to_owned()),
        span: item.span,
        params: item.params.iter().map(|param| param.id).collect(),
        module_bindings: Vec::new(),
        ops: Vec::new(),
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis: Some(&analysis),
        self_shape: None,
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
    };
    context.lower_expr(body, &mut plan.ops);
    if super::falls_through(body) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}

fn lower_struct_member_functions<'a>(
    module: &'a Module,
    resolved: &'a ResolvedModule,
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
                    StructMember::Callable(callable) => {
                        lower_callable_member(module, resolved, item, callable, param_shapes)
                    }
                    StructMember::IndexAccess(access) => {
                        lower_index_access_member(module, resolved, item, access, param_shapes)
                    }
                    StructMember::Field(_) | StructMember::SequenceMaterializer(_) => None,
                })
        })
}

fn lower_callable_member(
    module: &Module,
    resolved: &ResolvedModule,
    owner: &Item,
    callable: &CallableMember,
    param_shapes: &[(tune_hir::MemberId, Shape)],
) -> Option<PlanFunction> {
    let body = callable.body.as_ref()?;
    let analysis = tune_shape::analyze_item(module, resolved, owner);
    let mut plan = PlanFunction {
        owner: Some(owner.id),
        member: Some(callable.id),
        name: format!(
            "{}.{}",
            owner.name.as_deref().unwrap_or("<anonymous>"),
            callable.name.as_deref().unwrap_or("<anonymous>")
        ),
        span: callable.span,
        params: std::iter::once(callable.id)
            .chain(callable.params.iter().map(|param| param.id))
            .collect(),
        module_bindings: Vec::new(),
        ops: Vec::new(),
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis: Some(&analysis),
        self_shape: owner.name.as_ref().map(|name| Shape::Struct(name.clone())),
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
    };
    context.lower_expr(body, &mut plan.ops);
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
    param_shapes: &[(tune_hir::MemberId, Shape)],
) -> Option<PlanFunction> {
    let body = access.body.as_ref()?;
    let analysis = tune_shape::analyze_item(module, resolved, owner);
    let mut plan = PlanFunction {
        owner: Some(owner.id),
        member: Some(access.id),
        name: format!("{}.[index]", owner.name.as_deref().unwrap_or("<anonymous>")),
        span: access.span,
        params: vec![access.id, access.index_param_id],
        module_bindings: Vec::new(),
        ops: Vec::new(),
    };
    let context = LowerContext {
        resolved: Some(resolved),
        module: Some(module),
        analysis: Some(&analysis),
        self_shape: owner.name.as_ref().map(|name| Shape::Struct(name.clone())),
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: param_shapes.to_vec(),
    };
    context.lower_expr(body, &mut plan.ops);
    if super::falls_through(body) {
        plan.ops.push(PlanOp::Return);
    }
    Some(plan)
}
