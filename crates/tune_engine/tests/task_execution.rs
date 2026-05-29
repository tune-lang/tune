use tune_runtime::value::Value;

#[test]
fn run_file_executes_spawn_join_ready_value() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn 20
let result: Int = task.join()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(20));
    Ok(())
}

#[test]
fn run_file_executes_direct_call_inside_spawn_body() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let helper(): Int = 41
let task: Task<Int> = spawn helper()
let result: Int = task.join() + 1
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(42));
    Ok(())
}

#[test]
fn run_file_executes_spawn_body_with_captured_param() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let start(seed: Int): Int = {
  let task: Task<Int> = spawn seed + 1
  task.join()
}
let result: Int = start(4)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(5));
    Ok(())
}

#[test]
fn run_file_does_not_execute_spawned_work_before_join() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn panic("deferred")
let result: Int = 1
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(1));
    Ok(())
}

#[test]
fn run_file_reports_spawned_panic_at_join() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn panic("joined")
let result: Int = task.join()
"#,
        )
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
        return Err("joining a panicked task should report diagnostics");
    };

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains("joined"))
    }));

    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
