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
    assert!(report.functions[1].ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::DirectCall {
            target: tune_hir::HirId(0),
            arg_count: 1,
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
fn executable_lowering_failures_use_structured_diagnostics() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("main.tn", "let value: Int = ~1")
        .ok_or("source should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.executable_file(file) else {
        return Err("unsupported executable lowering should report diagnostics");
    };

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        tune_diagnostics::codes::EXECUTABLE_LOWERING_ERROR
    );
    assert!(
        diagnostics[0]
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains("UnsupportedOp"))
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
