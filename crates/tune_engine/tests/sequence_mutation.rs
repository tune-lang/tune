#[test]
fn run_file_executes_exclusive_sequence_mutation() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let result: Int = {
  let values = [1, 2]
  values[0] = 9
  values[0]
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::Value::Int(9));

    Ok(())
}

#[test]
fn run_file_executes_shared_cow_sequence_mutation() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let result: Int = {
  let values = [1, 2]
  let alias = values
  values[0] = 9
  alias[0]
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, tune_runtime::Value::Int(1));

    Ok(())
}

fn run_file(
    tune: &tune_engine::Tune,
    file: tune_db::FileId,
) -> Result<tune_runtime::Value, &'static str> {
    tune.run_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
