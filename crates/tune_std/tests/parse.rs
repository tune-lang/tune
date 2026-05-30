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

#[test]
fn parse_radix_executors_return_result_values() -> Result<(), &'static str> {
    let module = tune_std::parse::install();
    let executor = |name: &str| {
        module
            .functions
            .iter()
            .find(|function| function.name == name)
            .and_then(|function| function.executor.as_ref())
            .ok_or("parse radix function should carry an executor")
    };

    assert_eq!(
        executor("int_radix")?
            .call(&[
                tune_runtime::Value::String("ff".into()),
                tune_runtime::Value::Size(16),
            ])
            .map_err(|_| "parse.int_radix should execute")?,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Int(255)],
            propagation_frames: Vec::new(),
        }
    );
    assert_eq!(
        executor("size_radix")?
            .call(&[
                tune_runtime::Value::String("100".into()),
                tune_runtime::Value::Size(2),
            ])
            .map_err(|_| "parse.size_radix should execute")?,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Size(4)],
            propagation_frames: Vec::new(),
        }
    );
    assert_eq!(
        executor("byte_radix")?
            .call(&[
                tune_runtime::Value::String("ff".into()),
                tune_runtime::Value::Size(16),
            ])
            .map_err(|_| "parse.byte_radix should execute")?,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Byte(255)],
            propagation_frames: Vec::new(),
        }
    );

    let invalid_radix = executor("int_radix")?
        .call(&[
            tune_runtime::Value::String("10".into()),
            tune_runtime::Value::Size(1),
        ])
        .map_err(|_| "parse.int_radix should execute")?;
    assert!(matches!(
        invalid_radix,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultError,
            ..
        }
    ));

    Ok(())
}
