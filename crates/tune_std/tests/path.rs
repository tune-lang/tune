fn path_executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("path function should carry an executor")
}

#[test]
fn path_sequence_helpers_return_strings() -> Result<(), &'static str> {
    let module = tune_std::path::install();

    let joined = path_executor(&module, "join_all")?
        .call(&[tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::String("src".into()),
            tune_runtime::Value::String("main.tn".into()),
        ])])
        .map_err(|_| "path.join_all should execute")?;
    assert_eq!(
        joined,
        tune_runtime::Value::String(std::path::Path::new("src/main.tn").display().to_string())
    );

    let components = path_executor(&module, "components")?
        .call(&[tune_runtime::Value::String("src/main.tn".into())])
        .map_err(|_| "path.components should execute")?;
    assert_eq!(
        components,
        tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::String("src".into()),
            tune_runtime::Value::String("main.tn".into()),
        ])
    );

    Ok(())
}

#[test]
fn path_predicates_and_separator_execute() -> Result<(), &'static str> {
    let module = tune_std::path::install();

    let relative = path_executor(&module, "is_relative")?
        .call(&[tune_runtime::Value::String("src/main.tn".into())])
        .map_err(|_| "path.is_relative should execute")?;
    assert_eq!(relative, tune_runtime::Value::Bool(true));

    let separator = path_executor(&module, "separator")?
        .call(&[])
        .map_err(|_| "path.separator should execute")?;
    assert_eq!(
        separator,
        tune_runtime::Value::String(std::path::MAIN_SEPARATOR.to_string())
    );

    Ok(())
}
