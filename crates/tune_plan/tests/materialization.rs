#[test]
fn plan_uses_shape_analysis_materialization_facts() -> Result<(), &'static str> {
    let source = "let result: Size = 3";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);
    let plan = tune_plan::lower_analyzed_module_item_to_plan(
        &module,
        &module.items[0],
        &resolved,
        &analysis,
    )
    .ok_or("item should lower")?;

    assert!(
        plan.ops
            .iter()
            .any(|op| { matches!(op, tune_plan::PlanOp::ConstSize { value: 3 }) })
    );

    Ok(())
}
