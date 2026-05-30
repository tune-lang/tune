fn executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("encoding function should carry an executor")
}

#[test]
fn encoding_module_exposes_task_safe_helpers() -> Result<(), &'static str> {
    let module = tune_std::encoding::install();

    for name in ["hex", "from_hex"] {
        let function = module
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or("encoding function should be installed")?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
    }

    Ok(())
}

#[test]
fn encoding_hex_executors_round_trip_bytes() -> Result<(), &'static str> {
    let module = tune_std::encoding::install();
    let encoded = executor(&module, "hex")?
        .call(&[tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::Byte(0),
            tune_runtime::Value::Byte(10),
            tune_runtime::Value::Byte(255),
        ])])
        .map_err(|_| "encoding.hex should execute")?;
    assert_eq!(encoded, tune_runtime::Value::String("000aff".into()));

    let decoded = executor(&module, "from_hex")?
        .call(&[tune_runtime::Value::String("000AFF".into())])
        .map_err(|_| "encoding.from_hex should execute")?;
    assert_eq!(
        decoded,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Sequence(vec![
                tune_runtime::Value::Byte(0),
                tune_runtime::Value::Byte(10),
                tune_runtime::Value::Byte(255),
            ])],
            propagation_frames: Vec::new(),
        }
    );

    let invalid = executor(&module, "from_hex")?
        .call(&[tune_runtime::Value::String("abc".into())])
        .map_err(|_| "encoding.from_hex should execute")?;
    assert!(matches!(
        invalid,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultError,
            ..
        }
    ));

    Ok(())
}
