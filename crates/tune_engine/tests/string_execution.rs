#[test]
fn run_file_executes_string_len_and_index() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let result: String = {
  let text: String = "héllo"
  let index: Size = 1
  "{text[index]}:{text.len()}"
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        run_file(&tune, file)?,
        tune_runtime::value::Value::String("é:5".into())
    );
    Ok(())
}

#[test]
fn run_file_uses_unicode_scalar_string_indexing_policy() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let result: String = {
  let text: String = "é"
  let index: Size = 1
  "{text[index]}:{text.len()}"
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        run_file(&tune, file)?,
        tune_runtime::value::Value::String("\u{301}:2".into())
    );
    Ok(())
}

fn run_file(
    tune: &tune_engine::Tune,
    file: tune_db::FileId,
) -> Result<tune_runtime::value::Value, &'static str> {
    tune.run_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
