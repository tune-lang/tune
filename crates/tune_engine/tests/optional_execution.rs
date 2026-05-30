use tune_runtime::Value;

#[test]
fn run_file_narrows_optional_present_branch() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let value: Int? = 41
let result: Int = if value is not none {
  value + 1
} else {
  0
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(42));
    Ok(())
}

#[test]
fn run_file_narrows_optional_none_else_branch() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let value: Int? = 41
let result: Int = if value is none {
  0
} else {
  value + 1
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(42));
    Ok(())
}

#[test]
fn run_file_allows_optional_copy_warning() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let maybe(): Int? = none
let x: Int? = maybe()
let y = x
let result: Int = 1
"#,
        )
        .ok_or("file should allocate")?;
    let check = tune.check_source(file).ok_or("file should check")?;

    assert!(check.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == tune_diagnostics::Severity::Warning
            && diagnostic.title == "optional value may be none"
    }));
    assert_eq!(run_file(&tune, file)?, Value::Int(1));

    Ok(())
}

#[test]
fn run_file_rejects_proven_none_optional_copy() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let x: Int?
let y = x
"#,
        )
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_source(file) else {
        return Err("proven-none optional copy should stop execution");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == tune_diagnostics::Severity::Error
            && diagnostic.title == "optional value is proven none"
    }));

    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
