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
    assert!(report.module_plan.entry.is_none());
    assert!(
        report.functions[1]
            .ops
            .contains(&tune_plan::PlanOp::DirectCall {
                target: tune_hir::HirId(0),
                arg_count: 1,
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
fn executable_file_uses_module_entry_plan() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let helper(): Int = 99\nlet value: Int = 1 + 2")
        .ok_or("file should allocate")?;

    let executable = tune
        .executable_entry(tune_engine::EntryPoint::File(file))
        .map_err(|_| "file should lower to executable")?;

    assert_eq!(executable.compile.functions.len(), 1);
    assert_eq!(executable.ir.len(), 1);
    assert_eq!(executable.bytecode.entry_function, Some(0));
    assert_eq!(executable.ir[0].name, "<entry>");

    Ok(())
}

#[test]
fn run_file_executes_top_level_value_bindings_in_order() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let a: Int = 1\nlet b: Int = a + 2")
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
fn run_file_executes_direct_callable_invocation() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            "let add(a: Int, b: Int): Int = a + b\nlet value: Int = add(1, 2)",
        )
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
fn run_file_executes_explicit_return_from_callable() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            "let id(value: Int): Int = { return value; 99 }\nlet result: Int = id(3)",
        )
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
fn run_file_executes_if_return_from_callable() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let pick(flag: Bool): Int = {
  if flag {
    return 1
  };
  2
}
let result: Int = pick(true)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(1)
    );

    Ok(())
}

#[test]
fn run_file_executes_local_binding_slice_through_vm() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let value: Int = { let x: Int = 1; x + 2 }")
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_entry(tune_engine::EntryPoint::File(file))
            .map_err(|error| {
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
