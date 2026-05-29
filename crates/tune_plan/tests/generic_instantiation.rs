#[test]
fn direct_call_plan_preserves_solved_generic_type_args() -> Result<(), &'static str> {
    let source = r#"
let id<T>(value: T): T = value
let result: Int = id(1)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);
    let plan = tune_plan::lower_analyzed_module_item_to_plan(
        &module,
        &module.items[1],
        &resolved,
        &analysis,
    )
    .ok_or("item should lower")?;

    assert!(plan.ops.iter().any(|op| {
        matches!(
            op,
            tune_plan::PlanOp::DirectCall {
                target: tune_hir::HirId(0),
                type_args,
                ..
            } if type_args == &vec![tune_shape::Shape::Int]
        )
    }));

    Ok(())
}
