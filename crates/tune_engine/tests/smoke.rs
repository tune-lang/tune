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
    assert_eq!(report.entry_function, None);
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
fn compile_source_uses_module_aware_member_lowering() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let report = tune
        .compile_source(
            "main.tn",
            r#"
struct Stack {
  len(): Size = 0
  Stack[index: Size]: Int = index
}
let first(items: Stack) = items[0]
"#,
        )
        .map_err(|_| "engine should compile source")?;

    assert!(report.check.diagnostics.is_empty());
    assert!(report.functions[0].ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::SequenceGet {
            index_member: Some(_),
            ..
        }
    )));

    Ok(())
}

#[test]
fn run_file_executes_tiny_integer_file_entry_through_vm() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let helper(): Int = 99\nlet value: Int = 1 + 2")
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(3)
    );

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
        .load_project(dyno_project::manifest::Manifest::new("demo", "main.tn"))
        .map_err(|_| "project should load")?;

    assert_eq!(handle, tune_engine::ProjectHandle(0));
    assert_eq!(tune.projects().len(), 1);

    Ok(())
}
