#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

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
}

#[test]
fn std_host_installs_default_modules() {
    let modules = tune_std::modules();

    assert!(modules.iter().any(|module| module.name == "io"));
    assert!(modules.iter().any(|module| module.name == "parse"));
    assert!(modules.iter().any(|module| module.name == "text"));
    assert!(modules.iter().any(|module| module.name == "fs"));
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
