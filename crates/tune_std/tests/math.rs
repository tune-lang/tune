fn math_function<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostFunction, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .ok_or("math function should be installed")
}

fn call_float(
    module: &tune_host::HostModule,
    name: &str,
    args: Vec<tune_runtime::Value>,
) -> Result<f64, &'static str> {
    let executor = math_function(module, name)?
        .executor
        .as_ref()
        .ok_or("math function should carry an executor")?;
    match executor
        .call(&args)
        .map_err(|_| "math function should execute")?
    {
        tune_runtime::Value::Float(value) => Ok(value),
        _ => Err("math function should return Float"),
    }
}

fn call_bool(
    module: &tune_host::HostModule,
    name: &str,
    args: Vec<tune_runtime::Value>,
) -> Result<bool, &'static str> {
    let executor = math_function(module, name)?
        .executor
        .as_ref()
        .ok_or("math function should carry an executor")?;
    match executor
        .call(&args)
        .map_err(|_| "math function should execute")?
    {
        tune_runtime::Value::Bool(value) => Ok(value),
        _ => Err("math function should return Bool"),
    }
}

#[test]
fn math_module_exposes_task_safe_float_helpers() -> Result<(), &'static str> {
    let module = tune_std::math::install();

    for name in [
        "pi", "e", "abs", "min", "max", "clamp", "floor", "ceil", "round", "sqrt", "pow", "sin",
        "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "ln", "log10",
    ] {
        let function = math_function(&module, name)?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
        assert_eq!(function.ret, tune_shape::Shape::Float);
    }
    for name in ["is_finite", "is_nan", "is_infinite"] {
        let function = math_function(&module, name)?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
        assert_eq!(function.ret, tune_shape::Shape::Bool);
    }

    Ok(())
}

#[test]
fn math_executors_return_float_results() -> Result<(), &'static str> {
    let module = tune_std::math::install();

    assert!((call_float(&module, "pi", Vec::new())? - std::f64::consts::PI).abs() < f64::EPSILON);
    assert!((call_float(&module, "e", Vec::new())? - std::f64::consts::E).abs() < f64::EPSILON);
    assert_eq!(
        call_float(&module, "abs", vec![tune_runtime::Value::Float(-2.5)])?,
        2.5
    );
    assert_eq!(
        call_float(
            &module,
            "min",
            vec![
                tune_runtime::Value::Float(2.0),
                tune_runtime::Value::Float(3.0)
            ]
        )?,
        2.0
    );
    assert_eq!(
        call_float(
            &module,
            "max",
            vec![
                tune_runtime::Value::Float(2.0),
                tune_runtime::Value::Float(3.0)
            ]
        )?,
        3.0
    );
    assert_eq!(
        call_float(
            &module,
            "clamp",
            vec![
                tune_runtime::Value::Float(10.0),
                tune_runtime::Value::Float(1.0),
                tune_runtime::Value::Float(4.0),
            ]
        )?,
        4.0
    );
    assert_eq!(
        call_float(&module, "floor", vec![tune_runtime::Value::Float(3.75)])?,
        3.0
    );
    assert_eq!(
        call_float(&module, "ceil", vec![tune_runtime::Value::Float(3.25)])?,
        4.0
    );
    assert_eq!(
        call_float(&module, "round", vec![tune_runtime::Value::Float(3.5)])?,
        4.0
    );
    assert_eq!(
        call_float(&module, "sqrt", vec![tune_runtime::Value::Float(9.0)])?,
        3.0
    );
    assert_eq!(
        call_float(
            &module,
            "pow",
            vec![
                tune_runtime::Value::Float(2.0),
                tune_runtime::Value::Float(3.0)
            ]
        )?,
        8.0
    );
    assert!(
        (call_float(&module, "sin", vec![tune_runtime::Value::Float(0.0)])? - 0.0).abs() < 0.00001
    );
    assert!(
        (call_float(&module, "cos", vec![tune_runtime::Value::Float(0.0)])? - 1.0).abs() < 0.00001
    );
    assert!(
        (call_float(
            &module,
            "atan2",
            vec![
                tune_runtime::Value::Float(0.0),
                tune_runtime::Value::Float(1.0)
            ]
        )? - 0.0)
            .abs()
            < 0.00001
    );
    assert!(
        (call_float(&module, "exp", vec![tune_runtime::Value::Float(0.0)])? - 1.0).abs() < 0.00001
    );
    assert!(
        (call_float(&module, "ln", vec![tune_runtime::Value::Float(1.0)])? - 0.0).abs() < 0.00001
    );
    assert!(
        (call_float(&module, "log10", vec![tune_runtime::Value::Float(100.0)])? - 2.0).abs()
            < 0.00001
    );

    Ok(())
}

#[test]
fn math_classification_helpers_return_bools() -> Result<(), &'static str> {
    let module = tune_std::math::install();

    assert!(call_bool(
        &module,
        "is_finite",
        vec![tune_runtime::Value::Float(1.0)]
    )?);
    assert!(call_bool(
        &module,
        "is_nan",
        vec![tune_runtime::Value::Float(f64::NAN)]
    )?);
    assert!(call_bool(
        &module,
        "is_infinite",
        vec![tune_runtime::Value::Float(f64::INFINITY)]
    )?);

    Ok(())
}
