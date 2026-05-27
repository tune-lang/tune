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

#[test]
fn compile_source_returns_semantic_plans() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let report = tune
        .compile_source(
            "main.tn",
            r#"
let helper(value) = value
let run(input) = helper(input)
"#,
        )
        .map_err(|_| "engine should compile source")?;

    assert!(report.check.diagnostics.is_empty());
    assert_eq!(report.functions.len(), 2);
    assert!(
        report.functions[1]
            .ops
            .contains(&tune_plan::PlanOp::DirectCall {
                target: tune_hir::HirId(0)
            })
    );

    Ok(())
}

#[test]
fn runtime_entry_points_exist_without_claiming_vm_is_done() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("main.tn", "let run() = 1")
        .ok_or("file should allocate")?;

    assert!(matches!(
        tune.run_file(file),
        Err(tune_engine::EngineError::NotImplemented(
            "typed bytecode lowering and VM execution"
        ))
    ));

    Ok(())
}

#[test]
fn registers_host_modules_and_project_manifests() -> Result<(), &'static str> {
    struct EmptyHost;

    impl tune_host::Host for EmptyHost {}

    let mut tune = tune_engine::Tune::new();
    let registration = tune.register_host(&EmptyHost);
    assert_eq!(registration.module_count, 0);
    assert!(tune.host_modules().is_empty());

    let handle = tune
        .load_project(dyno_project::manifest::Manifest {
            name: "demo".to_owned(),
            edition: "2026".to_owned(),
            entry: "main.tn".to_owned(),
        })
        .map_err(|_| "project should load")?;

    assert_eq!(handle, tune_engine::ProjectHandle(0));
    assert_eq!(tune.projects().len(), 1);

    Ok(())
}
