#[test]
fn run_file_executes_while_local_mutation() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = {
  let i: Int = 0
  while i < 3 {
    i = i + 1
  }
  i
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(3));

    Ok(())
}

#[test]
fn run_file_executes_integer_comparison_operators() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let eq: Int = if 3 == 3 { 1 } else { 0 }
let ne: Int = if 3 ~= 4 { 1 } else { 0 }
let le: Int = if 3 <= 3 { 1 } else { 0 }
let ge: Int = if 4 >= 3 { 1 } else { 0 }
let result: Int = eq + ne + le + ge
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(4));

    Ok(())
}

#[test]
fn run_file_executes_integer_arithmetic_operators() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = ((20 - 4) * 3 / 2) % 10
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(4));

    Ok(())
}

#[test]
fn run_file_reports_integer_divide_by_zero_as_vm_fault() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let result: Int = 1 / 0")
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
        return Err("divide by zero should report diagnostics");
    };

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].facts.iter().any(|fact| {
        fact.entries
            .iter()
            .any(|entry| entry.message.contains("DivideByZero"))
    }));

    Ok(())
}

#[test]
fn run_file_executes_loop_break_and_continue() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = {
  let i: Int = 0
  let total: Int = 0
  loop {
    i = i + 1
    if i < 3 {
      continue
    }
    total = total + i
    if i >= 5 {
      break
    }
  }
  total
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(12));

    Ok(())
}

#[test]
fn run_file_executes_unary_negation_and_not() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let offset: Int = -4
let gate: Int = if not false { 10 } else { 0 }
let result: Int = gate + offset
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(6));

    Ok(())
}

#[test]
fn run_file_executes_integer_bit_not() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let result: Int = ~1")
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(!1));

    Ok(())
}

#[test]
fn run_file_executes_integer_bit_ops_and_shifts() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = {
  let a: Int = 6 & 3
  let b: Int = a | 8
  let c: Int = 1 << 2
  let d: Int = 16 >> 2
  (b ^ c) + d
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(18));

    Ok(())
}

#[test]
fn run_file_executes_boolean_short_circuit_ops() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let fail(): Bool = panic("short circuit")
let result: Int = {
  let left: Int = if true or fail() { 10 } else { 0 }
  let right: Int = if false and fail() { 0 } else { 5 }
  left + right
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(15));

    Ok(())
}

#[test]
fn run_file_executes_finite_for_over_sequence() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = {
  let values = [1, 2, 3, 4]
  let total: Int = 0
  for item in values {
    if item < 3 {
      continue
    }
    total = total + item
    if item >= 4 {
      break
    }
  }
  total
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(7));

    Ok(())
}

#[test]
fn run_file_executes_finite_for_over_ranges() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = {
  let total: Int = 0
  for item in 1..=4 {
    total = total + item
  }
  for item in 4..6 {
    total = total + item
  }
  total
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(19));

    Ok(())
}

#[test]
fn run_file_executes_finite_for_over_struct_contract() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Window {
  values: [Int]

  len(): Size = 3
  Window[index: Size]: Int = self.values[index]
}

let result: Int = {
  let window: Window = Window { values = [2, 4, 6] }
  let total: Int = 0
  for item in window {
    total = total + item
  }
  total
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(12));

    Ok(())
}

#[test]
fn run_file_executes_sequence_get_and_set() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let result: Int = {
  let values = [10, 20, 30]
  values[1] = 7
  values[0] + values[1] + values[2]
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::value::Value::Int(47));

    Ok(())
}

#[test]
fn run_file_reports_panic_with_message() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", r#"let result: Int = panic("bad")"#)
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
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

    Ok(())
}

fn run_file(
    tune: &tune_engine::Tune,
    file: tune_db::FileId,
) -> Result<tune_runtime::value::Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
