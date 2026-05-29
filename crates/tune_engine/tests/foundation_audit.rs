use tune_runtime::value::Value;

#[test]
fn run_file_solves_numeric_literal_binding_to_float_assignment() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result = {
  let x = 0
  x = 2.5
  x
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Float(2.5));
    Ok(())
}

#[test]
fn run_file_materializes_size_and_byte_scalars() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let size_file = tune
        .add_file("size.tn", "let start: Size = 1\nlet result: Size = start")
        .ok_or("file should allocate")?;
    let byte_file = tune
        .add_file("byte.tn", "let b: Byte = 255\nlet result: Byte = b + 1")
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, size_file)?, Value::Size(1));
    assert_eq!(run_file(&tune, byte_file)?, Value::Byte(0));
    Ok(())
}

#[test]
fn run_file_executes_is_aliases() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let a: Int = if 3 is 3 { 1 } else { 0 }
let b: Int = if 3 is not 4 { 1 } else { 0 }
let result: Int = a + b
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(2));
    Ok(())
}

#[test]
fn run_file_executes_simple_string_interpolation() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let name: String = "Tune"
let count: Int = 3
let result: String = "hello {name} {count}"
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        run_file(&tune, file)?,
        Value::String("hello Tune 3".to_owned())
    );
    Ok(())
}

#[test]
fn run_file_preserves_private_callable_capture_state() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = {
  let x: Int = 0
  let f = _(): Int = {
    x = x + 1
    x
  }
  let a: Int = f()
  let b: Int = f()
  x + b
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(2));
    Ok(())
}

#[test]
fn run_file_captures_structs_by_private_snapshot() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
  bump(): Int = {
    self.value = self.value + 1
    self.value
  }
}
let c: Counter = Counter { value = 0 }
let f = _(): Int = c.bump()
let a: Int = f()
let b: Int = f()
let result: Int = c.value + b
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(2));
    Ok(())
}

#[test]
fn run_file_captures_read_only_structs_by_reference() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
  bump(): Int = {
    self.value = self.value + 1
    self.value
  }
}
let c: Counter = Counter { value = 0 }
let f = _(): Int = c.value
let ignored: Int = c.bump()
let result: Int = f()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(1));
    Ok(())
}

#[test]
fn run_file_keeps_read_only_struct_captures_referenced_after_calls() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
  bump(): Int = {
    self.value = self.value + 1
    self.value
  }
}
let c: Counter = Counter { value = 0 }
let f = _(): Int = c.value
let before: Int = f()
let ignored: Int = c.bump()
let after: Int = f()
let result: Int = before + after
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(1));
    Ok(())
}

#[test]
fn run_file_snapshots_captured_structs_passed_to_calls() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
  bump(): Int = {
    self.value = self.value + 1
    self.value
  }
}
let touch(counter: Counter): Int = counter.bump()
let c: Counter = Counter { value = 0 }
let f = _(): Int = touch(c)
let inner: Int = f()
let result: Int = c.value + inner
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(1));
    Ok(())
}

#[test]
fn run_file_executes_mixed_structural_match_calls() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Duck {
  quack(): Int = 7
}
struct Rock {}
let speak(x) = match x {
  { quack(): Int } => quack()
  else 0
}
let duck: Duck = Duck {}
let rock: Rock = Rock {}
let a: Int = speak(duck)
let result: Int = a + speak(rock)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(7));
    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
