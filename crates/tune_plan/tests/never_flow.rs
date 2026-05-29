#[test]
fn never_shaped_call_body_does_not_get_implicit_return() -> Result<(), &'static str> {
    let source = r#"
let stop(): Never = panic("bad")
let fail(): Never = stop()
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analyses = tune_shape::analyze_module(&module, &resolved);
    let plan = tune_plan::lower_analyzed_module_to_plan(&module, &resolved, &analyses);
    let fail = plan
        .functions
        .iter()
        .find(|function| function.name == "fail")
        .ok_or("expected fail plan")?;

    assert!(
        fail.ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::DirectCall { .. }))
    );
    assert!(
        !fail
            .ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::Return))
    );

    Ok(())
}
