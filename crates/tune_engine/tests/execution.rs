#[test]
fn run_file_executes_tiny_integer_file_entry_through_vm() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let helper(): Int = 99\nlet value: Int = 1 + 2")
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}

#[test]
fn executable_file_uses_module_entry_plan() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let helper(): Int = 99\nlet value: Int = 1 + 2")
        .ok_or("file should allocate")?;

    let executable = tune
        .executable_entry(tune_engine::EntryPoint::File(file))
        .map_err(|_| "file should lower to executable")?;

    assert_eq!(executable.compile.functions.len(), 1);
    assert_eq!(executable.ir.len(), 1);
    assert_eq!(executable.bytecode.entry_function, Some(0));
    assert_eq!(executable.ir[0].name, "<entry>");

    Ok(())
}

#[test]
fn run_file_executes_top_level_value_bindings_in_order() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let a: Int = 1\nlet b: Int = a + 2")
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}

#[test]
fn run_file_executes_direct_callable_invocation() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            "let add(a: Int, b: Int): Int = a + b\nlet value: Int = add(1, 2)",
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}

#[test]
fn run_file_executes_explicit_return_from_callable() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            "let id(value: Int): Int = { return value; 99 }\nlet result: Int = id(3)",
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}

#[test]
fn run_file_executes_if_return_from_callable() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let pick(flag: Bool): Int = {
  if flag {
    return 1
  }
  2
}
let result: Int = pick(true)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(1)
    );

    Ok(())
}

#[test]
fn run_file_executes_if_expression_value() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            "let pick(flag: Bool): Int = if flag { 1 } else { 2 }\nlet result: Int = pick(false)",
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(2)
    );

    Ok(())
}

#[test]
fn run_file_executes_comparison_fed_if_expression() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            "let pick(value: Int): Int = if value > 10 { 1 } else { 2 }\nlet result: Int = pick(20)",
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(1)
    );

    Ok(())
}

#[test]
fn run_file_executes_elif_expression_value() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let pick(value: Int): Int = if value > 10 { 1 } elif value > 5 { 2 } else { 3 }
let result: Int = pick(7)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(2)
    );

    Ok(())
}

#[test]
fn run_file_executes_branch_local_assignment() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let pick(flag: Bool): Int = {
  let result: Int = 0
  if flag {
    result = 1
  } else {
    result = 2
  }
  result
}
let value: Int = pick(false)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(2)
    );

    Ok(())
}

#[test]
fn run_file_executes_result_propagation_ok_path() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let pass(): Result<Int, Int> = {
  let value: Int = Ok(1)!
  Ok(value)
}
let result = pass()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::value::Value::Int(1)],
            propagation_frames: Vec::new(),
        }
    );

    Ok(())
}

#[test]
fn run_file_executes_result_propagation_error_path() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let fail(): Result<Int, Int> = {
  let value: Int = Error(2)!
  Ok(value)
}
let result = fail()
"#,
        )
        .ok_or("file should allocate")?;

    let value = tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })?;
    let tune_runtime::value::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultError,
        fields,
        propagation_frames,
    } = value
    else {
        return Err("result should be Error variant");
    };
    assert_eq!(fields, vec![tune_runtime::value::Value::Int(2)]);
    assert_eq!(propagation_frames.len(), 1);
    assert_eq!(propagation_frames[0].function, 1);
    assert_eq!(propagation_frames[0].function_name, "fail");
    assert!(propagation_frames[0].span.is_some());
    let result_error = tune_runtime::value::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultError,
        fields,
        propagation_frames,
    };
    let diagnostic = tune_engine::diagnostic_from_result_error(&result_error)
        .ok_or("result error should produce a propagation diagnostic")?;
    assert_eq!(
        diagnostic.code,
        tune_diagnostics::codes::RESULT_PROPAGATION_ERROR
    );
    assert!(diagnostic.facts.iter().any(|fact| {
        fact.entries
            .iter()
            .any(|entry| entry.message.contains("propagated through `fail`"))
    }));
    let source_diagnostic =
        tune_engine::diagnostic_from_result_error_with_sources(&result_error, tune.db())
            .ok_or("result error should produce a source-aware propagation diagnostic")?;
    assert!(source_diagnostic.facts.iter().any(|fact| {
        fact.entries.iter().any(|entry| {
            entry
                .message
                .contains("propagated through `fail` at `Error(2)!`")
        })
    }));

    Ok(())
}

#[test]
fn run_file_executes_spawn_join_ready_value() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
let task: Task<Int> = spawn 20
let result: Int = task.join()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "file entry should run"
        })?,
        tune_runtime::value::Value::Int(20)
    );
    Ok(())
}

#[test]
fn run_file_executes_local_binding_slice_through_vm() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file("app.tn", "let value: Int = { let x: Int = 1; x + 2 }")
        .ok_or("file should allocate")?;

    assert_eq!(
        tune.run_entry(tune_engine::EntryPoint::File(file))
            .map_err(|error| {
                eprintln!("{error:?}");
                "file entry should run"
            })?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}
