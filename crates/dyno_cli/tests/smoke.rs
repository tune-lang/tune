#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn package_exposes_dyno_binary() {
    assert!(option_env!("CARGO_BIN_EXE_dyno").is_some());
}

#[test]
fn renders_engine_diagnostics_with_shared_renderer() {
    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(0),
        tune_diagnostics::ByteOffset::new(1),
        tune_diagnostics::ByteOffset::new(3),
    );
    let diagnostic = tune_diagnostics::Diagnostic::error(
        tune_diagnostics::codes::RUNTIME_ERROR,
        "runtime execution failed",
        span,
        "execution failed here",
    )
    .build();
    let rendered =
        dyno_cli::render_engine_error(&tune_engine::EngineError::Diagnostics(vec![diagnostic]));

    assert_eq!(rendered.len(), 1);
    assert!(rendered[0].contains("error[T0903]: runtime execution failed"));
}

#[test]
fn renders_unhandled_result_error_at_runtime_boundary() -> Result<(), &'static str> {
    let mut db = tune_db::TuneDb::new();
    let file = db
        .add_file("main.tn", "let value = load()!")
        .ok_or("source should allocate")?;
    let span = tune_diagnostics::Span::new(
        file,
        tune_diagnostics::ByteOffset::new(12),
        tune_diagnostics::ByteOffset::new(19),
    );
    let value = tune_runtime::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultError,
        fields: vec![tune_runtime::Value::Int(1)],
        propagation_frames: vec![tune_runtime::PropagationFrame {
            function: 2,
            instruction: 7,
            function_name: "load".to_owned(),
            span: Some(span),
        }],
    };
    let rendered = dyno_cli::render_runtime_boundary(&value);

    assert_eq!(rendered.len(), 1);
    assert!(rendered[0].contains("error[T0901]: result error propagated"));
    assert!(rendered[0].contains("propagated through `load`"));

    let rendered = dyno_cli::render_runtime_boundary_with_sources(&value, &db);
    assert_eq!(rendered.len(), 1);
    assert!(rendered[0].contains("propagated through `load` at `load()!`"));

    Ok(())
}
