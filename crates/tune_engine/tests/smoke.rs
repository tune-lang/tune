#[test]
fn checks_source_through_engine_facade() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let report = tune
        .check_source(
            "main.tn",
            r#"
tag tool {}
@tool
let run(input: String): String = input
"#,
        )
        .ok_or("engine should check source")?;

    assert!(report.diagnostics.is_empty());
    assert_eq!(report.module.items.len(), 2);
    assert!(report.resolved.scope.get("run").is_some());

    Ok(())
}
