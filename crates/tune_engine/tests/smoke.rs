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
fn checks_source_from_path_through_engine_facade() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("tune-engine-path-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let path = root.join("main.tn");
    std::fs::write(&path, "let value: Int = 42").map_err(|error| error.to_string())?;

    let mut tune = tune_engine::Tune::new();
    let report = tune
        .check_path(&path)
        .map_err(|error| format!("{error:?}"))?;
    std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;

    assert!(report.diagnostics.is_empty());
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
    assert!(report.functions[1].ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::DirectCall {
            target: tune_hir::HirId(0),
            arg_count: 1,
            type_args: _,
            span: Some(_),
        }
    )));

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
fn engine_resolves_loaded_project_roots() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let project = tune
        .load_project(dyno_project::Manifest::new("app", "main.tn"))
        .map_err(|_| "project should load")?;
    let resolution = tune
        .resolve_project(project, &dyno_project::Lockfile::new())
        .map_err(|_| "project should resolve")?;

    assert!(resolution.roots.contains(&dyno_project::ModuleRoot::Std));
    assert_eq!(resolution.locked_package_count, 0);
    Ok(())
}

#[test]
fn engine_loads_and_runs_manifest_entry_source() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/helper.tn".to_owned(),
                    "let ignored: Int = 1".to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    "let result: Int = 40 + 2".to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry)
            .map_err(|_| "project entry should run")?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_can_import_member_from_loaded_source() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    "let add(a: Int, b: Int): Int = a + b".to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".add
let result: Int = add(20, 22)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry)
            .map_err(|_| "project entry should run")?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_reports_unresolved_import_members() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                ("src/math.tn".to_owned(), "let add(a, b) = a + b".to_owned()),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".missing
let result = missing(1, 2)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_project_entry(entry)
    else {
        return Err("unresolved import member should stop execution");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::UNRESOLVED_NAME
            && diagnostic.title == "unresolved import member `missing`"
    }));

    Ok(())
}

#[test]
fn executable_lowering_stops_on_structured_frontend_diagnostics() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("main.tn", "let value: Int = true & false")
        .ok_or("source should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.executable_file(file) else {
        return Err("frontend diagnostics should stop executable lowering");
    };

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == tune_diagnostics::codes::SHAPE_MISMATCH)
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
    assert_eq!(registration.function_count, 0);
    assert!(tune.host_modules().is_empty());
    assert!(tune.host_symbols().is_empty());

    let handle = tune
        .load_project(dyno_project::manifest::Manifest::new("demo", "main.tn"))
        .map_err(|_| "project should load")?;

    assert_eq!(handle, tune_engine::ProjectHandle(0));
    assert_eq!(tune.projects().len(), 1);

    Ok(())
}

#[test]
fn builder_style_host_registration_is_available() {
    struct EmptyHost;

    impl tune_host::Host for EmptyHost {}

    let tune = tune_engine::Tune::new().with_host(&EmptyHost);

    assert!(tune.host_modules().is_empty());
    assert!(tune.host_symbols().is_empty());
}

#[test]
fn registered_host_functions_get_stable_engine_symbols() -> Result<(), &'static str> {
    struct FsHost;

    impl tune_host::Host for FsHost {
        fn modules(&self) -> Vec<tune_host::HostModule> {
            vec![tune_host::HostModule::new(
                "fs",
                vec![tune_host::HostFunction::new(
                    "read",
                    vec![tune_host::HostParam::new("path", tune_shape::Shape::String)],
                    tune_shape::Shape::String,
                )],
            )]
        }
    }

    let mut tune = tune_engine::Tune::new();
    let registration = tune.register_host(&FsHost);

    assert_eq!(registration.module_count, 1);
    assert_eq!(registration.function_count, 1);
    assert_eq!(tune.host_symbols().len(), 1);
    assert_eq!(
        tune.host_symbols()[0].id,
        tune_engine::EngineHostSymbolId(0)
    );
    assert_eq!(tune.host_symbols()[0].module, "fs");
    assert_eq!(tune.host_symbols()[0].function, "read");
    assert_eq!(
        tune.host_symbol(tune_engine::EngineHostSymbolId(0))
            .ok_or("symbol should exist")?,
        &tune.host_symbols()[0]
    );

    Ok(())
}

#[test]
fn engine_registers_default_std_host_modules() {
    let mut tune = tune_engine::Tune::new();
    let registration = tune.register_std();

    assert_eq!(registration.module_count, 4);
    assert_eq!(registration.function_count, 11);
    assert!(tune.host_modules().iter().any(|module| module.name == "io"));
    assert!(
        tune.host_symbols()
            .iter()
            .any(|symbol| symbol.module == "parse" && symbol.function == "int")
    );
    assert!(
        tune.host_symbols()
            .iter()
            .any(|symbol| symbol.module == "fs" && symbol.function == "read_text")
    );
}

#[test]
fn engine_runs_imported_std_host_function() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_std();
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "parse".int
let value: Result<Int, String> = int("42")
"#,
        )
        .ok_or("source should allocate")?;

    let value = tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "std host import should execute"
    })?;

    assert_eq!(
        value,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Int(42)],
            propagation_frames: Vec::new(),
        }
    );

    Ok(())
}

#[test]
fn vm_faults_convert_to_structured_diagnostics() {
    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(3),
        tune_diagnostics::ByteOffset::new(8),
        tune_diagnostics::ByteOffset::new(13),
    );
    let fault = tune_vm::VmFault::new(
        tune_vm::VmError::UnsupportedOpcode(tune_bytecode::Opcode::AddInt),
        Some(tune_vm::VmLocation {
            function: 2,
            function_name: Some("add".to_owned()),
            instruction: Some(5),
            span: Some(span),
        }),
    );

    let diagnostic = tune_engine::diagnostic_from_vm_fault(&fault);

    assert_eq!(diagnostic.code, tune_diagnostics::codes::RUNTIME_ERROR);
    assert_eq!(diagnostic.primary.span, span);
    assert!(
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message == "bytecode instruction: 5")
    );
    assert!(
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message == "fault in `add`")
    );
}

#[test]
fn vm_fault_diagnostics_can_include_source_summary() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("main.tn", "let value = 1 + true")
        .ok_or("source should allocate")?;
    let span = tune_diagnostics::Span::new(
        file,
        tune_diagnostics::ByteOffset::new(12),
        tune_diagnostics::ByteOffset::new(20),
    );
    let fault = tune_vm::VmFault::new(
        tune_vm::VmError::UnsupportedOpcode(tune_bytecode::Opcode::AddInt),
        Some(tune_vm::VmLocation {
            function: 0,
            function_name: Some("<entry>".to_owned()),
            instruction: Some(3),
            span: Some(span),
        }),
    );

    let diagnostic = tune_engine::diagnostic_from_vm_fault_with_sources(&fault, tune.db());

    assert!(
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message == "fault in `<entry>` at `1 + true`")
    );

    Ok(())
}
