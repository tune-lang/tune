#[test]
fn stdcore_registry_includes_auto_included_core_shapes() {
    let registry = tune_std::prelude::stdcore();

    assert!(
        registry
            .shapes
            .contains(&tune_std::prelude::StdCoreShape::Result)
    );
    assert!(
        registry
            .shapes
            .contains(&tune_std::prelude::StdCoreShape::Map)
    );
    assert!(
        registry
            .shapes
            .contains(&tune_std::prelude::StdCoreShape::Set)
    );
    assert!(
        registry
            .functions
            .contains(&tune_std::prelude::StdCoreFunction::Print)
    );
    assert_eq!(
        tune_std::prelude::StdCoreFunction::Print.host_function(),
        Some(tune_std::prelude::StdCoreHostFunction {
            module: "io",
            function: "print",
        })
    );
}

#[test]
fn std_host_installs_default_modules() {
    let modules = tune_std::modules();

    assert!(modules.iter().any(|module| module.name == "io"));
    assert!(modules.iter().any(|module| module.name == "math"));
    assert!(modules.iter().any(|module| module.name == "bits"));
    assert!(modules.iter().any(|module| module.name == "parse"));
    assert!(modules.iter().any(|module| module.name == "text"));
    assert!(modules.iter().any(|module| module.name == "path"));
    assert!(modules.iter().any(|module| module.name == "env"));
    assert!(modules.iter().any(|module| module.name == "fs"));
    assert!(modules.iter().any(|module| module.name == "hash"));
    assert!(modules.iter().any(|module| module.name == "json"));
    assert!(modules.iter().any(|module| module.name == "process"));
    assert!(modules.iter().any(|module| module.name == "random"));
    assert!(modules.iter().any(|module| module.name == "time"));
}

#[test]
fn parse_int_executor_returns_result_values() -> Result<(), &'static str> {
    let module = tune_std::parse::install();
    let function = module
        .functions
        .iter()
        .find(|function| function.name == "int")
        .ok_or("parse.int should be installed")?;
    let executor = function
        .executor
        .as_ref()
        .ok_or("parse.int should carry an executor")?;

    let value = executor
        .call(&[tune_runtime::Value::String("42".into())])
        .map_err(|_| "parse.int should execute")?;

    assert_eq!(
        value,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Int(42)],
            propagation_frames: Vec::new(),
        }
    );

    Ok(())
}

#[test]
fn std_host_reports_missing_string_arguments_before_type_mismatch() -> Result<(), &'static str> {
    let module = tune_std::text::install();
    let executor = module
        .functions
        .iter()
        .find(|function| function.name == "contains")
        .and_then(|function| function.executor.as_ref())
        .ok_or("text.contains should carry an executor")?;

    let error = match executor.call(&[tune_runtime::Value::String("hello".into())]) {
        Ok(_) => return Err("missing needle should be a host call error"),
        Err(error) => error,
    };

    assert!(error.message.contains("missing argument `needle`"));
    assert!(error.message.contains("index 1"));
    Ok(())
}

#[test]
fn fs_text_executors_return_result_values() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let write = module
        .functions
        .iter()
        .find(|function| function.name == "write_text")
        .and_then(|function| function.executor.as_ref())
        .ok_or("fs.write_text should carry an executor")?;
    let read = module
        .functions
        .iter()
        .find(|function| function.name == "read_text")
        .and_then(|function| function.executor.as_ref())
        .ok_or("fs.read_text should carry an executor")?;

    let path = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}.txt",
        std::process::id(),
        "fs-text"
    ));
    let path_text = path.to_string_lossy().to_string();

    let write_result = write
        .call(&[
            tune_runtime::Value::String(path_text.clone()),
            tune_runtime::Value::String("hello std".into()),
        ])
        .map_err(|_| "fs.write_text should execute")?;
    assert!(matches!(
        write_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));

    let read_result = read
        .call(&[tune_runtime::Value::String(path_text)])
        .map_err(|_| "fs.read_text should execute")?;
    assert_eq!(
        read_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::String("hello std".into())],
            propagation_frames: Vec::new(),
        }
    );

    drop(std::fs::remove_file(path));
    Ok(())
}

#[test]
fn env_module_exposes_typed_process_read_functions() -> Result<(), &'static str> {
    let module = tune_std::env::install();

    let args = module
        .functions
        .iter()
        .find(|function| function.name == "args")
        .ok_or("env.args should be installed")?;
    assert!(matches!(args.ret, tune_shape::Shape::Sequence(_)));
    assert!(
        args.authorities
            .iter()
            .any(|authority| authority.0 == "env.read")
    );

    let get = module
        .functions
        .iter()
        .find(|function| function.name == "get")
        .ok_or("env.get should be installed")?;
    assert!(matches!(get.ret, tune_shape::Shape::Optional(_)));
    assert!(
        get.authorities
            .iter()
            .any(|authority| authority.0 == "env.read")
    );

    let cwd = module
        .functions
        .iter()
        .find(|function| function.name == "cwd")
        .ok_or("env.cwd should be installed")?;
    assert!(matches!(cwd.ret, tune_shape::Shape::Result { .. }));
    assert!(
        cwd.authorities
            .iter()
            .any(|authority| authority.0 == "env.read")
    );

    Ok(())
}

#[test]
fn env_get_executor_returns_optional_string() -> Result<(), &'static str> {
    let module = tune_std::env::install();
    let executor = module
        .functions
        .iter()
        .find(|function| function.name == "get")
        .and_then(|function| function.executor.as_ref())
        .ok_or("env.get should carry an executor")?;

    let missing_name = format!("DYNO_TUNE_STD_MISSING_{}", std::process::id());
    assert_eq!(
        executor
            .call(&[tune_runtime::Value::String(missing_name)])
            .map_err(|_| "env.get should execute")?,
        tune_runtime::Value::None
    );

    Ok(())
}

#[test]
fn env_cwd_executor_returns_result_value() -> Result<(), &'static str> {
    let module = tune_std::env::install();
    let executor = module
        .functions
        .iter()
        .find(|function| function.name == "cwd")
        .and_then(|function| function.executor.as_ref())
        .ok_or("env.cwd should carry an executor")?;

    let value = executor.call(&[]).map_err(|_| "env.cwd should execute")?;
    assert!(matches!(
        value,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));

    Ok(())
}

#[test]
fn path_module_exposes_task_safe_pure_helpers() -> Result<(), &'static str> {
    let module = tune_std::path::install();

    for name in ["join", "ext", "stem", "parent", "normalize"] {
        let function = module
            .functions
            .iter()
            .find(|function| function.name == name)
            .ok_or("path helper should be installed")?;
        assert!(function.task_safe);
        assert!(function.authorities.is_empty());
    }

    let join = module
        .functions
        .iter()
        .find(|function| function.name == "join")
        .ok_or("path.join should be installed")?;
    assert_eq!(join.ret, tune_shape::Shape::String);

    let ext = module
        .functions
        .iter()
        .find(|function| function.name == "ext")
        .ok_or("path.ext should be installed")?;
    assert!(matches!(ext.ret, tune_shape::Shape::Optional(_)));

    Ok(())
}

#[test]
fn path_executors_return_string_and_optional_values() -> Result<(), &'static str> {
    let module = tune_std::path::install();
    let executor = |name: &str| {
        module
            .functions
            .iter()
            .find(|function| function.name == name)
            .and_then(|function| function.executor.as_ref())
            .ok_or("path helper should carry an executor")
    };

    assert_eq!(
        executor("join")?
            .call(&[
                tune_runtime::Value::String("src".into()),
                tune_runtime::Value::String("main.tn".into()),
            ])
            .map_err(|_| "path.join should execute")?,
        tune_runtime::Value::String(
            std::path::Path::new("src")
                .join("main.tn")
                .display()
                .to_string()
        )
    );
    assert_eq!(
        executor("ext")?
            .call(&[tune_runtime::Value::String("src/main.tn".into())])
            .map_err(|_| "path.ext should execute")?,
        tune_runtime::Value::String("tn".into())
    );
    assert_eq!(
        executor("ext")?
            .call(&[tune_runtime::Value::String("README".into())])
            .map_err(|_| "path.ext should execute")?,
        tune_runtime::Value::None
    );
    assert_eq!(
        executor("stem")?
            .call(&[tune_runtime::Value::String("src/main.tn".into())])
            .map_err(|_| "path.stem should execute")?,
        tune_runtime::Value::String("main".into())
    );
    assert_eq!(
        executor("parent")?
            .call(&[tune_runtime::Value::String("src/main.tn".into())])
            .map_err(|_| "path.parent should execute")?,
        tune_runtime::Value::String("src".into())
    );
    assert_eq!(
        executor("normalize")?
            .call(&[tune_runtime::Value::String("src/./tools/../main.tn".into())])
            .map_err(|_| "path.normalize should execute")?,
        tune_runtime::Value::String(
            std::path::Path::new("src")
                .join("main.tn")
                .display()
                .to_string()
        )
    );

    Ok(())
}

#[test]
fn text_contains_executor_returns_bool() -> Result<(), &'static str> {
    let module = tune_std::text::install();
    let executor = module
        .functions
        .iter()
        .find(|function| function.name == "contains")
        .and_then(|function| function.executor.as_ref())
        .ok_or("text.contains should carry an executor")?;

    assert_eq!(
        executor
            .call(&[
                tune_runtime::Value::String("hello std".into()),
                tune_runtime::Value::String("std".into()),
            ])
            .map_err(|_| "text.contains should execute")?,
        tune_runtime::Value::Bool(true)
    );

    Ok(())
}
