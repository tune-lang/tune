use tune_runtime::Value;

#[test]
fn run_file_executes_generic_callable_with_multiple_instantiations() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let id<T>(value: T): T = value
let left: Int = id(2)
let right: String = id("x")
let result: String = "{left}:{right}"
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::String("2:x".into()));
    Ok(())
}

#[test]
fn run_file_executes_generic_struct_field_access() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
struct Box<T> {
  value: T
}
let read<T>(box: Box<T>): T = box.value
let int_value: Int = read(Box { value = 4 })
let string_value: String = read(Box { value = "ok" })
let result: String = "{int_value}:{string_value}"
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::String("4:ok".into()));
    Ok(())
}

#[test]
fn executable_preserves_generic_function_arity_metadata() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let id<T>(value: T): T = value
let result: Int = id(2)
"#,
        )
        .ok_or("file should allocate")?;

    let executable = tune.executable_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file should compile"
    })?;
    let generic = executable
        .bytecode
        .functions
        .iter()
        .find(|function| function.name == "id")
        .ok_or("generic function should lower")?;
    assert_eq!(generic.generic_param_count, 1);
    let call_site = executable
        .bytecode
        .functions
        .iter()
        .flat_map(|function| &function.call_sites)
        .find(|site| site.type_args == vec![tune_shape::Shape::Int])
        .ok_or("generic direct call should carry type args")?;
    assert_eq!(
        call_site.generic_strategy,
        tune_bytecode::function::BytecodeGenericStrategy::DirectSpecialization
    );

    Ok(())
}

#[test]
fn executable_marks_forwarded_generic_call_as_shared_witness() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let id<T>(value: T): T = value
let wrap<T>(value: T): T = id(value)
let result: Int = wrap(2)
"#,
        )
        .ok_or("file should allocate")?;

    let executable = tune.executable_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file should compile"
    })?;
    let wrap = executable
        .bytecode
        .functions
        .iter()
        .find(|function| function.name == "wrap")
        .ok_or("wrap function should lower")?;
    let forwarded = wrap
        .call_sites
        .iter()
        .find(|site| site.type_args == vec![tune_shape::Shape::Param("T".into())])
        .ok_or("forwarded generic call should carry param type arg")?;
    assert_eq!(
        forwarded.generic_strategy,
        tune_bytecode::function::BytecodeGenericStrategy::WitnessShared
    );

    Ok(())
}

#[test]
fn check_file_rejects_unsolved_generic_call_type_args() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let make<T>(): T = panic("unsolved")
let result = make()
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_source(file).ok_or("file should check")?;

    assert!(check.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "generic call type arguments could not be inferred"
    }));

    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
