use tune_runtime::value::Value;

#[test]
fn run_file_executes_match_on_result_variant() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let input: Result = Ok(1)
let result: Int = match input {
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
        .add_source(
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
        .add_source(
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
        .add_source(
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

#[test]
fn run_file_executes_nested_enum_pattern_match() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
enum Inner {
  Num(Int)
  Text(Int)
}
enum Outer {
  Wrap(Inner)
}
let outer: Outer = Wrap(Text(7))
let result: Int = match outer {
  Wrap(Num(value)) => value + 1
  Wrap(Text(value)) => value + 2
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(9));
    Ok(())
}

#[test]
fn run_file_executes_tuple_pattern_match() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let pair = (2, 5)
let result: Int = match pair {
  (left, right) => left + right
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(7));
    Ok(())
}

#[test]
fn run_file_executes_tuple_pattern_nested_in_variant() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
enum Wrapped {
  Pair((Int, Int))
}
let wrapped: Wrapped = Pair((3, 4))
let result: Int = match wrapped {
  Pair((left, right)) => left * right
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(12));
    Ok(())
}

#[test]
fn run_file_executes_none_pattern_match() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let value: Int? = none
let result: Int = match value {
  none => 1
  else 2
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(1));
    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
