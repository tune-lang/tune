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

fn run_file(
    tune: &tune_engine::Tune,
    file: tune_db::FileId,
) -> Result<tune_runtime::value::Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
