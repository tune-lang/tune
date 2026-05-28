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

#[test]
fn run_file_executes_struct_member_call_with_self_receiver() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
  next(): Int = {
    self.value = self.value + 1
    self.value
  }
}
let counter: Counter = Counter {
  value = 1
}
let result: Int = counter.next()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(2));
    Ok(())
}

#[test]
fn run_file_executes_structural_match_member_witness() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Duck {
  quack(): Int = 7
}
let duck: Duck = Duck {}
let result: Int = match duck {
  { quack(): Int } => quack()
  else 0
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(7));
    Ok(())
}

#[test]
fn run_file_executes_structural_match_fallback_when_shape_does_not_match()
-> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Rock {}
let rock: Rock = Rock {}
let result: Int = match rock {
  { quack(): Int } => quack()
  else 0
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(0));
    Ok(())
}

#[test]
fn run_file_preserves_member_receiver_mutation_for_caller() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
  next(): Int = {
    self.value = self.value + 1
    self.value
  }
}
let counter: Counter = Counter {
  value = 1
}
let ignored: Int = counter.next()
let result: Int = counter.value
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(2));
    Ok(())
}

#[test]
fn run_file_constructs_struct_with_local_non_atomic_state() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
}
let result: Counter = Counter {
  value = 1
}
"#,
        )
        .ok_or("file should allocate")?;

    let Value::Struct { state, .. } = run_file(&tune, file)? else {
        return Err("entry should return a struct");
    };

    assert_eq!(state.repr, tune_runtime::StateRepr::LocalHandle);
    assert_eq!(
        state.ownership,
        tune_runtime::ownership::OwnershipPlan::NonAtomicRc
    );
    Ok(())
}

#[test]
fn run_file_spawned_struct_uses_shared_atomic_state() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
struct Counter {
  value: Int
}
let task: Task<Counter> = spawn Counter {
  value = 1
}
let result: Counter = task.join()
"#,
        )
        .ok_or("file should allocate")?;

    let Value::Struct { state, .. } = run_file(&tune, file)? else {
        return Err("entry should return a struct");
    };

    assert_eq!(state.repr, tune_runtime::StateRepr::SharedHandle);
    assert_eq!(
        state.ownership,
        tune_runtime::ownership::OwnershipPlan::SharedAtomic
    );
    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
