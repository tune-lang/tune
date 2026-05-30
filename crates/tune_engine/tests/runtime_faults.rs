#[test]
fn run_file_reports_proven_integer_divide_by_zero_before_execution() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source("app.tn", "let result: Int = 1 / 0")
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_source(file) else {
        return Err("divide by zero should report diagnostics");
    };

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        tune_diagnostics::codes::NUMERIC_OVERFLOW
    );

    Ok(())
}

#[test]
fn run_file_reports_panic_with_message() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source("app.tn", r#"let result: Int = panic("bad")"#)
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_source(file) else {
        return Err("panic should report a runtime diagnostic");
    };

    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0]
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains("bad"))
    );
    assert!(
        diagnostics[0]
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains(r#"panic("bad")"#))
    );

    Ok(())
}
