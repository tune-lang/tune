use tune_runtime::Value;

#[test]
fn run_file_executes_generic_callable_with_multiple_instantiations() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
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
        .add_file(
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
        .add_file(
            "app.tn",
            r#"
let id<T>(value: T): T = value
let result: Int = id(2)
"#,
        )
        .ok_or("file should allocate")?;

    let executable = tune.executable_file(file).map_err(|error| {
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
    assert!(
        executable
            .bytecode
            .functions
            .iter()
            .flat_map(|function| &function.call_sites)
            .any(|site| site.type_args == vec![tune_shape::Shape::Int])
    );

    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
