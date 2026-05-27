use tune_runtime::value::Value;

#[test]
fn run_file_executes_struct_literal_field_get() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct User {
  age: Int
}
let user: User = User {
  age = 20
}
let result: Int = user.age
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(20));
    Ok(())
}

#[test]
fn run_file_executes_struct_field_set_on_local() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct User {
  age: Int
}
let result: Int = {
  let user: User = User {
    age = 20
  }
  user.age = 21
  user.age
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(21));
    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
