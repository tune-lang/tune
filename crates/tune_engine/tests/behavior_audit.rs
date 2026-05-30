use tune_diagnostics::{Severity, codes};
use tune_runtime::Value;

#[test]
fn match_hole_fallback_diagnostic_has_structured_help() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let result: Int = match 1 {
  _ => 0
}
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_source(file).ok_or("file should check")?;
    let diagnostic = check
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == codes::MATCH_HOLE_FALLBACK)
        .ok_or("expected match hole fallback diagnostic")?;

    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.primary.message.contains("else"));
    assert!(
        diagnostic
            .helps
            .iter()
            .any(|help| help.message.contains("else"))
    );

    let rendered = tune_diagnostics::render::render_plain(diagnostic);
    assert!(rendered.contains("error[T0804]"));
    assert!(rendered.contains("help:"));

    Ok(())
}

#[test]
fn structural_match_does_not_introduce_synthetic_branch_locals() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
struct Duck {
  quack(): Int = 1
}
let speak(duck) = match duck {
  { quack(): Int } => quack()
  else 0
}
let result: Int = speak(Duck {})
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_source(file).ok_or("file should check")?;

    assert!(check.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error && diagnostic.code == codes::UNRESOLVED_NAME
    }));

    Ok(())
}

#[test]
fn direct_owned_struct_self_cycle_is_rejected_by_shape_analysis() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
struct Node {
  next: Node?
}
let node: Node = Node { next = none }
node.next = node
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_source(file).ok_or("file should check")?;

    assert!(check.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error && diagnostic.code == codes::SELF_STATE_ERROR
    }));

    Ok(())
}

#[test]
fn aliased_owned_struct_cycle_is_rejected_before_field_set() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
struct Node {
  next: Node?
}
let result: Int = {
  let node: Node = Node { next = none }
  let alias = node
  node.next = alias
  1
}
"#,
        )
        .ok_or("file should allocate")?;

    let error = tune
        .run_source(file)
        .err()
        .ok_or("expected runtime error")?;
    let rendered = format!("{error:?}");
    assert!(rendered.contains("RecursiveStructState"), "{rendered}");

    Ok(())
}

#[test]
fn missing_else_default_hole_solves_from_expected_shape() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let pick(flag: Bool): Int = if flag { 3 }
let result: Int = pick(false)
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(0));
    Ok(())
}

#[test]
fn annotated_bindings_without_initializers_use_shape_defaults() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let top: Int
let result: Int = {
  let local: Int;
  top + local
}
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(0));
    Ok(())
}

#[test]
fn struct_field_default_solves_from_non_literal_member_assignment() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let make_float(): Float = 2.5
struct Example {
  x = 0
  change(): Float = {
    self.x = make_float()
    self.x
  }
}
let item: Example = Example {}
let result: Float = item.change()
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Float(2.5));
    Ok(())
}

#[test]
fn bare_result_annotation_solves_generic_payloads_from_match_flow() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let choose(okay: Bool): Result = if okay { Ok(41) } else { Error("bad") }
let selected: Result = choose(true)
let result: Int = match selected { Ok(value) => value + 1; Error(_) => 0 }
"#,
        )
        .ok_or("file should allocate")?;

    assert_eq!(run_file(&tune, file)?, Value::Int(42));
    Ok(())
}

#[test]
fn runtime_integer_overflow_reports_diagnostic_not_host_panic() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let max(): Int = 9223372036854775807
let result: Int = max() + 1
"#,
        )
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_source(file) else {
        return Err("runtime overflow should return diagnostics");
    };

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error && diagnostic.code == codes::RUNTIME_ERROR
    }));

    Ok(())
}

#[test]
fn sequence_out_of_bounds_reports_runtime_diagnostic() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let values: [Int] = [1, 2]
let result: Int = values[2]
"#,
        )
        .ok_or("file should allocate")?;

    assert_runtime_error(tune.run_source(file))
}

#[test]
fn string_out_of_bounds_reports_runtime_diagnostic() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let text: String = "hi"
let result: String = text[text.len()]
"#,
        )
        .ok_or("file should allocate")?;

    assert_runtime_error(tune.run_source(file))
}

#[test]
fn assignment_shape_mismatch_diagnostic_keeps_materialization_context() -> Result<(), &'static str>
{
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_source(
            "app.tn",
            r#"
let result = {
  let x = 1
  x = "bad"
  x
}
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_source(file).ok_or("file should check")?;
    let diagnostic = check
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == codes::ASSIGNMENT_SHAPE_MISMATCH)
        .ok_or("expected assignment mismatch diagnostic")?;

    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(!diagnostic.facts.is_empty());
    assert!(
        diagnostic
            .helps
            .iter()
            .any(|help| help.message.contains("shadow"))
    );

    Ok(())
}

fn assert_runtime_error(
    result: Result<Value, tune_engine::EngineError>,
) -> Result<(), &'static str> {
    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = result else {
        return Err("runtime fault should return diagnostics");
    };

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error && diagnostic.code == codes::RUNTIME_ERROR
    }));

    Ok(())
}

fn run_file(tune: &tune_engine::Tune, file: tune_db::FileId) -> Result<Value, &'static str> {
    tune.run_source(file).map_err(|error| {
        eprintln!("{error:?}");
        "file entry should run"
    })
}
