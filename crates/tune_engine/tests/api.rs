#[test]
fn public_api_uses_file_names_for_paths() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("tune-engine-api-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let path = root.join("main.tn");
    std::fs::write(&path, "let value: Int = 40 + 2").map_err(|error| error.to_string())?;

    let mut tune = tune_engine::Tune::new();
    let value = tune
        .run_file(&path)
        .map_err(|error| format!("run_file should execute a path: {error:?}"))?;
    std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;

    assert_eq!(value, tune_runtime::Value::Int(42));
    Ok(())
}

#[test]
fn public_api_uses_source_names_for_loaded_sources() -> Result<(), String> {
    let mut tune = tune_engine::Tune::new();
    let source = tune
        .add_source("memory.tn", "let value: Int = 21 * 2")
        .ok_or("source should allocate")?;

    let check = tune
        .check_source(source)
        .ok_or("loaded source should check")?;
    assert!(check.diagnostics.is_empty());

    let value = tune
        .run_source(source)
        .map_err(|error| format!("run_source should execute loaded source: {error:?}"))?;
    assert_eq!(value, tune_runtime::Value::Int(42));
    Ok(())
}

#[test]
fn public_api_can_split_compile_and_runtime() -> Result<(), String> {
    let mut tune = tune_engine::Tune::new();
    let executable = tune
        .executable_text("memory.tn", "let value: Int = 6 * 7")
        .map_err(|error| format!("source text should compile: {error:?}"))?;

    let mut runtime = tune.runtime(executable);
    let value = runtime
        .run_entry()
        .map_err(|error| format!("runtime should execute entry: {error:?}"))?;

    assert_eq!(value, tune_runtime::Value::Int(42));
    Ok(())
}
