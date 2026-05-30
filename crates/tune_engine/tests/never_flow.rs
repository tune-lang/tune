#[test]
fn run_file_uses_user_never_function_as_non_continuing_branch() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let stop(): Never = panic("bad")
let result: Int = if false { stop() } else { 5 }
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_source(file).map_err(|error| {
            eprintln!("{error:?}");
            "file should run"
        })?,
        tune_runtime::value::Value::Int(5)
    );

    Ok(())
}
