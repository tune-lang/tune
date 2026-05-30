#[test]
fn package_exposes_dyno_binary() {
    assert!(option_env!("CARGO_BIN_EXE_dyno").is_some());
}

#[test]
fn parses_cli_commands_without_special_entry_names() {
    assert_eq!(
        dyno_cli::parse_command(&["main.tn".to_owned()]),
        Ok(dyno_cli::CliCommand::Run {
            path: Some("main.tn".to_owned()),
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["run".to_owned()]),
        Ok(dyno_cli::CliCommand::Run { path: None })
    );
    assert_eq!(
        dyno_cli::parse_command(&["build".to_owned()]),
        Ok(dyno_cli::CliCommand::Build { path: None })
    );
    assert_eq!(
        dyno_cli::parse_command(&["check".to_owned()]),
        Ok(dyno_cli::CliCommand::Check { path: None })
    );
    assert_eq!(
        dyno_cli::parse_command(&["check".to_owned(), "main.tn".to_owned()]),
        Ok(dyno_cli::CliCommand::Check {
            path: Some("main.tn".to_owned()),
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["build".to_owned(), "main.tn".to_owned()]),
        Ok(dyno_cli::CliCommand::Build {
            path: Some("main.tn".to_owned()),
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["profile".to_owned(), "main.tn".to_owned()]),
        Ok(dyno_cli::CliCommand::Profile {
            path: Some("main.tn".to_owned()),
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["profile".to_owned()]),
        Ok(dyno_cli::CliCommand::Profile { path: None })
    );
    assert_eq!(
        dyno_cli::parse_command(&["fmt".to_owned()]),
        Ok(dyno_cli::CliCommand::Fmt {
            path: None,
            check: false,
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["fmt".to_owned(), "main.tn".to_owned()]),
        Ok(dyno_cli::CliCommand::Fmt {
            path: Some("main.tn".to_owned()),
            check: false,
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["fmt".to_owned(), "--check".to_owned()]),
        Ok(dyno_cli::CliCommand::Fmt {
            path: None,
            check: true,
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["fmt".to_owned(), "--check".to_owned(), "main.tn".to_owned()]),
        Ok(dyno_cli::CliCommand::Fmt {
            path: Some("main.tn".to_owned()),
            check: true,
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["lsp".to_owned()]),
        Ok(dyno_cli::CliCommand::Lsp)
    );
    assert_eq!(
        dyno_cli::parse_command(&["explain".to_owned()]),
        Ok(dyno_cli::CliCommand::Explain { code: None })
    );
    assert_eq!(
        dyno_cli::parse_command(&["explain".to_owned(), "T0301".to_owned()]),
        Ok(dyno_cli::CliCommand::Explain {
            code: Some("T0301".to_owned()),
        })
    );
    assert_eq!(
        dyno_cli::parse_command(&["new".to_owned(), "app".to_owned()]),
        Ok(dyno_cli::CliCommand::New {
            name: "app".to_owned(),
        })
    );
    assert_eq!(dyno_cli::parse_command(&[]), Ok(dyno_cli::CliCommand::Help));
    assert!(dyno_cli::parse_command(&["bad".to_owned(), "main.tn".to_owned()]).is_err());
}

#[test]
fn renders_diagnostic_explanations() {
    let list = dyno_cli::render_explain(None);
    assert!(list.contains("T0301"));
    assert!(list.contains("shape mismatch"));

    let single = dyno_cli::render_explain(Some("T0804"));
    assert!(single.contains("match hole fallback"));
    assert!(single.contains("use `else`"));

    let unknown = dyno_cli::render_explain(Some("T9999"));
    assert!(unknown.contains("unknown diagnostic code"));
}

#[test]
fn formats_single_file_in_place() -> Result<(), String> {
    let path = std::env::temp_dir().join(format!("dyno-fmt-{}.tn", std::process::id()));
    std::fs::write(&path, "let value:Int=1\n").map_err(|error| error.to_string())?;

    assert!(dyno_cli::format_file(&path)?);
    let formatted = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    assert_eq!(formatted, "let value: Int = 1\n");
    assert!(!dyno_cli::file_needs_format(&path)?);
    assert!(!dyno_cli::format_file(&path)?);

    std::fs::remove_file(path).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn checks_project_format_without_writing() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("dyno-fmt-check-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
    }
    let project = dyno_cli::create_project_in(&root, "demo_app")?;
    std::fs::write(&project.entry, "let value:Int=1\n").map_err(|error| error.to_string())?;

    let unformatted = dyno_cli::check_format_project(&project.root)?;
    let source = std::fs::read_to_string(&project.entry).map_err(|error| error.to_string())?;

    std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;

    assert_eq!(unformatted, vec![project.entry]);
    assert_eq!(source, "let value:Int=1\n");

    Ok(())
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

#[test]
fn loads_project_sources_from_manifest() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("dyno-cli-load-project-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
    }
    let project = dyno_cli::create_project_in(&root, "demo_app")?;
    let loaded = dyno_cli::load_project_from_dir(&project.root)?;

    std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;

    assert_eq!(loaded.manifest.name, "demo_app");
    assert!(loaded.sources.iter().any(|(path, _)| path == "src/main.tn"));

    Ok(())
}

#[test]
fn creates_new_project_scaffold() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("dyno-cli-new-project-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;

    let project = dyno_cli::create_project_in(&root, "demo_app")?;
    let manifest = std::fs::read_to_string(&project.manifest).map_err(|error| error.to_string())?;
    let entry = std::fs::read_to_string(&project.entry).map_err(|error| error.to_string())?;

    std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;

    assert_eq!(project.name, "demo_app");
    assert!(manifest.contains("[project]"));
    assert!(manifest.contains("entry = \"src/main.tn\""));
    assert!(entry.contains("let message"));

    Ok(())
}

#[test]
fn renders_profile_report_sections() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source("main.tn", "let value: Int = 40 + 2")
        .ok_or("source should allocate")?;
    let report = tune
        .profile_source(file)
        .map_err(|_| "profile should be produced")?;
    let rendered = dyno_cli::render_profile_report(&report);

    assert!(rendered.contains("compile stages:"));
    assert!(rendered.contains("plan quality:"));
    assert!(rendered.contains("ir quality:"));
    assert!(rendered.contains("bytecode quality:"));
    assert!(report.stop_reason.is_none());

    Ok(())
}
