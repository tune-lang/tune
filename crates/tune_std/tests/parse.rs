#[test]
fn parse_bool_executor_returns_result_values() -> Result<(), &'static str> {
    let module = tune_std::parse::install();
    let executor = module
        .functions
        .iter()
        .find(|function| function.name == "bool")
        .and_then(|function| function.executor.as_ref())
        .ok_or("parse.bool should carry an executor")?;

    let ok = executor
        .call(&[tune_runtime::Value::String("true".into())])
        .map_err(|_| "parse.bool should execute")?;
    assert_eq!(
        ok,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Bool(true)],
            propagation_frames: Vec::new(),
        }
    );

    let error = executor
        .call(&[tune_runtime::Value::String("yes".into())])
        .map_err(|_| "parse.bool should execute")?;
    assert!(matches!(
        error,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultError,
            ..
        }
    ));

    Ok(())
}
