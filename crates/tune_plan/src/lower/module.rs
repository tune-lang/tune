use tune_hir::item::{Item, ItemKind};
use tune_hir::module::Module;
use tune_resolve::ResolvedModule;
use tune_shape::MaterializationPlan;

use super::{LowerContext, lower_resolved_module_item_to_plan};
use crate::plan::{PlanFunction, PlanModule, PlanOp};

#[must_use]
pub fn lower_resolved_module_to_plan(module: &Module, resolved: &ResolvedModule) -> PlanModule {
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
            name: "<entry>".to_owned(),
            module_bindings: module_bindings.clone(),
            ops: Vec::new(),
        };
        for item in module
            .items
            .iter()
            .filter(|item| item.kind == ItemKind::Let && item.body.is_some())
        {
            lower_module_item_into_entry(module, item, resolved, last_binding, &mut entry.ops);
        }
        entry.ops.push(PlanOp::Return);
        entry
    });
    let functions = module
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::CallableDecl)
        .filter_map(|item| lower_resolved_module_item_to_plan(module, item, resolved))
        .collect();

    PlanModule { entry, functions }
}

fn lower_module_item_into_entry(
    module: &Module,
    item: &Item,
    resolved: &ResolvedModule,
    last_binding: Option<tune_hir::HirId>,
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
