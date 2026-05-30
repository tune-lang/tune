fn bits_function<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostFunction, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .ok_or("bits function should be installed")
}

fn call(
    module: &tune_host::HostModule,
    name: &str,
    args: Vec<tune_runtime::Value>,
) -> Result<tune_runtime::Value, &'static str> {
    bits_function(module, name)?
        .executor
        .as_ref()
        .ok_or("bits function should carry an executor")?
        .call(&args)
        .map_err(|_| "bits function should execute")
}

#[test]
fn bits_module_exposes_task_safe_int_helpers() -> Result<(), &'static str> {
    let module = tune_std::bits::install();

    for name in [
        "count_ones",
        "leading_zeros",
        "trailing_zeros",
        "rotate_left",
        "rotate_right",
    ] {
        let function = bits_function(&module, name)?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
    }

    assert_eq!(
        bits_function(&module, "count_ones")?.ret,
        tune_shape::Shape::Size
    );
    assert_eq!(
        bits_function(&module, "rotate_left")?.ret,
        tune_shape::Shape::Int
    );

    Ok(())
}

#[test]
fn bits_executors_return_size_and_int_values() -> Result<(), &'static str> {
    let module = tune_std::bits::install();

    assert_eq!(
        call(
            &module,
            "count_ones",
            vec![tune_runtime::Value::Int(0b1011)]
        )?,
        tune_runtime::Value::Size(3)
    );
    assert_eq!(
        call(&module, "leading_zeros", vec![tune_runtime::Value::Int(1)])?,
        tune_runtime::Value::Size(63)
    );
    assert_eq!(
        call(
            &module,
            "trailing_zeros",
            vec![tune_runtime::Value::Int(0b1000)]
        )?,
        tune_runtime::Value::Size(3)
    );
    assert_eq!(
        call(
            &module,
            "rotate_left",
            vec![tune_runtime::Value::Int(1), tune_runtime::Value::Size(2),]
        )?,
        tune_runtime::Value::Int(4)
    );
    assert_eq!(
        call(
            &module,
            "rotate_right",
            vec![tune_runtime::Value::Int(4), tune_runtime::Value::Size(2),]
        )?,
        tune_runtime::Value::Int(1)
    );

    Ok(())
}
