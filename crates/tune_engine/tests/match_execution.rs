use tune_runtime::value::Value;

#[test]
fn run_file_executes_match_on_result_variant() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = match Ok(1) {
  Ok(value) => value
  Error(_) => 0
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(1));
    Ok(())
}

#[test]
fn run_file_executes_match_on_user_enum_variant() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
enum Choice {
  One(Int)
  Two(Int)
}
let choice: Choice = One(2)
let result: Int = match choice {
  One(value) => value
  Two(value) => value
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(2));
    Ok(())
}

#[test]
fn run_file_executes_match_on_user_enum_param() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
enum Choice {
  One(Int)
  Two(Int)
}
let pick(choice: Choice): Int = match choice {
  One(value) => value
  Two(value) => value
}
let result: Int = pick(Two(4))
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(4));
    Ok(())
}

#[test]
fn run_file_executes_direct_call_nested_in_match_arm() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
enum Choice {
  One(Int)
  Two(Int)
}
let add_one(value: Int): Int = value + 1
let choice: Choice = One(4)
let result: Int = match choice {
  One(value) => add_one(value)
  Two(value) => value
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(5));
    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
