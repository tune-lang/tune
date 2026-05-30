fn executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("random function should carry an executor")
}

#[test]
fn random_module_exposes_task_safe_deterministic_helpers() -> Result<(), &'static str> {
    let module = tune_std::random::install();

    for name in ["size", "float", "int", "bytes"] {
        let function = module
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or("random function should be installed")?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
    }

    Ok(())
}

#[test]
fn random_executors_are_deterministic() -> Result<(), &'static str> {
    let module = tune_std::random::install();

    let first = executor(&module, "size")?
        .call(&[tune_runtime::Value::Size(7), tune_runtime::Value::Size(0)])
        .map_err(|_| "random.size should execute")?;
    let second = executor(&module, "size")?
        .call(&[tune_runtime::Value::Size(7), tune_runtime::Value::Size(0)])
        .map_err(|_| "random.size should execute")?;
    let next = executor(&module, "size")?
        .call(&[tune_runtime::Value::Size(7), tune_runtime::Value::Size(1)])
        .map_err(|_| "random.size should execute")?;

    assert_eq!(first, second);
    assert_ne!(first, next);

    let float = executor(&module, "float")?
        .call(&[tune_runtime::Value::Size(7), tune_runtime::Value::Size(0)])
        .map_err(|_| "random.float should execute")?;
    let tune_runtime::Value::Float(float) = float else {
        return Err("random.float should return Float");
    };
    assert!((0.0..1.0).contains(&float));

    Ok(())
}

#[test]
fn random_result_executors_return_ranges_and_bytes() -> Result<(), &'static str> {
    let module = tune_std::random::install();

    let int_value = executor(&module, "int")?
        .call(&[
            tune_runtime::Value::Size(7),
            tune_runtime::Value::Size(0),
            tune_runtime::Value::Int(10),
            tune_runtime::Value::Int(20),
        ])
        .map_err(|_| "random.int should execute")?;
    assert!(matches!(
        int_value,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields,
            ..
        } if matches!(fields.as_slice(), [tune_runtime::Value::Int(value)] if (10..=20).contains(value))
    ));

    let invalid_range = executor(&module, "int")?
        .call(&[
            tune_runtime::Value::Size(7),
            tune_runtime::Value::Size(0),
            tune_runtime::Value::Int(20),
            tune_runtime::Value::Int(10),
        ])
        .map_err(|_| "random.int should execute")?;
    assert!(matches!(
        invalid_range,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultError,
            ..
        }
    ));

    let bytes = executor(&module, "bytes")?
        .call(&[tune_runtime::Value::Size(7), tune_runtime::Value::Size(4)])
        .map_err(|_| "random.bytes should execute")?;
    assert!(matches!(
        bytes,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields,
            ..
        } if matches!(fields.as_slice(), [tune_runtime::Value::Sequence(items)] if items.len() == 4)
    ));

    Ok(())
}
