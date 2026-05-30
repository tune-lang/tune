#[test]
fn process_module_exposes_authorized_run_function() -> Result<(), &'static str> {
    let module = tune_std::process::install();
    let function = module
        .functions
        .iter()
        .find(|function| function.name == "run")
        .ok_or("process.run should be installed")?;

    assert!(matches!(function.ret, tune_shape::Shape::Result { .. }));
    assert!(!function.task_safe);
    assert!(
        function
            .authorities
            .iter()
            .any(|authority| authority.0 == "process.run")
    );
    assert_eq!(module.values.len(), 1);
    assert_eq!(module.values[0].name, "ProcessResult");

    Ok(())
}

#[test]
fn process_run_executor_returns_error_for_missing_command() -> Result<(), &'static str> {
    let module = tune_std::process::install();
    let executor = module
        .functions
        .iter()
        .find(|function| function.name == "run")
        .and_then(|function| function.executor.as_ref())
        .ok_or("process.run should carry an executor")?;

    let value = executor
        .call(&[
            tune_runtime::Value::String(format!(
                "dyno-tune-missing-command-{}",
                std::process::id()
            )),
            tune_runtime::Value::Sequence(Vec::new()),
        ])
        .map_err(|_| "process.run should execute")?;

    assert!(matches!(
        value,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultError,
            ..
        }
    ));

    Ok(())
}
