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
    let shell = module
        .functions
        .iter()
        .find(|function| function.name == "shell")
        .ok_or("process.shell should be installed")?;
    assert!(matches!(shell.ret, tune_shape::Shape::Result { .. }));
    assert!(!shell.task_safe);
    assert!(
        shell
            .authorities
            .iter()
            .any(|authority| authority.0 == "process.run")
    );
    for name in [
        "success",
        "code",
        "stdout",
        "stderr",
        "stdout_lines",
        "stderr_lines",
    ] {
        let helper = module
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or("process result helper should be installed")?;
        assert!(helper.task_safe);
        assert!(helper.authorities.is_empty());
    }
    assert_eq!(module.values.len(), 1);
    assert_eq!(module.values[0].name, "ProcessResult");

    Ok(())
}

#[test]
fn process_result_helpers_read_host_struct_fields() -> Result<(), &'static str> {
    let module = tune_std::process::install();
    let executor = |name: &str| {
        module
            .functions
            .iter()
            .find(|function| function.name == name)
            .and_then(|function| function.executor.as_ref())
            .ok_or("process helper should carry an executor")
    };
    let result = tune_runtime::Value::HostStruct {
        type_name: "process.ProcessResult".into(),
        fields: vec![
            ("code".into(), tune_runtime::Value::Int(0)),
            (
                "stdout".into(),
                tune_runtime::Value::String("one\ntwo\n".into()),
            ),
            (
                "stderr".into(),
                tune_runtime::Value::String("warn\n".into()),
            ),
        ],
    };

    assert_eq!(
        executor("success")?
            .call(std::slice::from_ref(&result))
            .map_err(|_| "process.success should execute")?,
        tune_runtime::Value::Bool(true)
    );
    assert_eq!(
        executor("code")?
            .call(std::slice::from_ref(&result))
            .map_err(|_| "process.code should execute")?,
        tune_runtime::Value::Int(0)
    );
    assert_eq!(
        executor("stdout")?
            .call(std::slice::from_ref(&result))
            .map_err(|_| "process.stdout should execute")?,
        tune_runtime::Value::String("one\ntwo\n".into())
    );
    assert_eq!(
        executor("stderr")?
            .call(std::slice::from_ref(&result))
            .map_err(|_| "process.stderr should execute")?,
        tune_runtime::Value::String("warn\n".into())
    );
    assert_eq!(
        executor("stdout_lines")?
            .call(std::slice::from_ref(&result))
            .map_err(|_| "process.stdout_lines should execute")?,
        tune_runtime::Value::Sequence(vec![
            tune_runtime::Value::String("one".into()),
            tune_runtime::Value::String("two".into()),
        ])
    );
    assert_eq!(
        executor("stderr_lines")?
            .call(std::slice::from_ref(&result))
            .map_err(|_| "process.stderr_lines should execute")?,
        tune_runtime::Value::Sequence(vec![tune_runtime::Value::String("warn".into())])
    );

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
