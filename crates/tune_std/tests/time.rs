fn executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("time function should carry an executor")
}

#[test]
fn time_module_exposes_authorized_task_safe_functions() -> Result<(), &'static str> {
    let module = tune_std::time::install();

    let now = module
        .functions
        .iter()
        .find(|function| function.name == "now_millis")
        .ok_or("time.now_millis should be installed")?;
    assert!(now.task_safe);
    assert!(matches!(now.ret, tune_shape::Shape::Result { .. }));
    assert!(
        now.authorities
            .iter()
            .any(|authority| authority.0 == "time.read")
    );

    let monotonic = module
        .functions
        .iter()
        .find(|function| function.name == "monotonic_millis")
        .ok_or("time.monotonic_millis should be installed")?;
    assert!(monotonic.task_safe);
    assert_eq!(monotonic.ret, tune_shape::Shape::Size);
    assert!(
        monotonic
            .authorities
            .iter()
            .any(|authority| authority.0 == "time.read")
    );

    let sleep = module
        .functions
        .iter()
        .find(|function| function.name == "sleep_millis")
        .ok_or("time.sleep_millis should be installed")?;
    assert!(sleep.task_safe);
    assert!(matches!(sleep.ret, tune_shape::Shape::Result { .. }));
    assert!(
        sleep
            .authorities
            .iter()
            .any(|authority| authority.0 == "time.sleep")
    );

    Ok(())
}

#[test]
fn time_executors_return_runtime_values() -> Result<(), &'static str> {
    let module = tune_std::time::install();

    let now = executor(&module, "now_millis")?
        .call(&[])
        .map_err(|_| "time.now_millis should execute")?;
    assert!(matches!(
        now,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields,
            ..
        } if matches!(fields.as_slice(), [tune_runtime::Value::Int(value)] if *value > 0)
    ));

    let monotonic = executor(&module, "monotonic_millis")?
        .call(&[])
        .map_err(|_| "time.monotonic_millis should execute")?;
    assert!(matches!(monotonic, tune_runtime::Value::Size(_)));

    let sleep = executor(&module, "sleep_millis")?
        .call(&[tune_runtime::Value::Size(0)])
        .map_err(|_| "time.sleep_millis should execute")?;
    assert!(matches!(
        sleep,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields,
            ..
        } if matches!(fields.as_slice(), [tune_runtime::Value::Unit])
    ));

    Ok(())
}
