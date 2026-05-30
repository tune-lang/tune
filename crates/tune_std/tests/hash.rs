fn executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("hash function should carry an executor")
}

#[test]
fn hash_module_exposes_task_safe_helpers() -> Result<(), &'static str> {
    let module = tune_std::hash::install();

    for name in ["text", "bytes", "combine"] {
        let function = module
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or("hash function should be installed")?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
        assert_eq!(function.ret, tune_shape::Shape::Size);
    }

    Ok(())
}

#[test]
fn hash_executors_return_stable_values() -> Result<(), &'static str> {
    let module = tune_std::hash::install();

    let text = executor(&module, "text")?
        .call(&[tune_runtime::Value::String("Tune".into())])
        .map_err(|_| "hash.text should execute")?;
    let bytes = executor(&module, "bytes")?
        .call(&[tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::Byte(b'T'),
            tune_runtime::Value::Byte(b'u'),
            tune_runtime::Value::Byte(b'n'),
            tune_runtime::Value::Byte(b'e'),
        ])])
        .map_err(|_| "hash.bytes should execute")?;
    assert_eq!(text, bytes);

    let combined = executor(&module, "combine")?
        .call(&[text.clone(), tune_runtime::Value::Size(7)])
        .map_err(|_| "hash.combine should execute")?;
    assert_ne!(combined, text);

    Ok(())
}
