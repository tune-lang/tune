fn analyze_item(source: &str, index: usize) -> tune_shape::ShapeAnalysis {
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    tune_shape::analyze_item(&module, &resolved, &module.items[index])
}

#[test]
fn analyzer_binds_enum_pattern_payload_shapes() {
    let analysis = analyze_item(
        r#"
enum E {
  A(Int)
}
let e: E = A(1)
let result: String = match e {
  A(v) => v
}
"#,
        2,
    );

    assert!(
        analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
                && diagnostic.primary.message.contains("got `Int`")
        }),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn analyzer_substitutes_generic_enum_pattern_payload_shapes() {
    let analysis = analyze_item(
        r#"
enum Box<T> {
  Some(T)
}
let b: Box<String> = Some("x")
let result: Int = match b {
  Some(v) => v
}
"#,
        2,
    );

    assert!(
        analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
                && diagnostic.primary.message.contains("got `String`")
        }),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn analyzer_binds_tuple_pattern_field_shapes() {
    let analysis = analyze_item(
        r#"
let pair: (Int, String) = (1, "x")
let result: Int = match pair {
  (number, text) => text
}
"#,
        1,
    );

    assert!(
        analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
                && diagnostic.primary.message.contains("got `String`")
        }),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn analyzer_reports_non_exhaustive_result_match() {
    let analysis = analyze_item(
        r#"
let value: Result<Int, String> = Ok(1)
let result: Int = match value {
  Ok(v) => v
}
"#,
        1,
    );

    assert!(
        analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::MATCH_NOT_EXHAUSTIVE
                && diagnostic.primary.message.contains("Result")
        }),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn analyzer_binds_optional_present_pattern_to_payload_shape() {
    let analysis = analyze_item(
        r#"
let maybe: Int? = 1
let result: String = match maybe {
  none => "missing"
  value => value
}
"#,
        1,
    );

    assert!(
        analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
                && diagnostic.primary.message.contains("Int")
        }),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn analyzer_narrows_optional_truthiness_to_payload_shape() {
    let analysis = analyze_item(
        r#"
let maybe: Int? = 1
let result: String = if maybe {
  maybe
} else {
  "missing"
}
"#,
        1,
    );

    assert!(
        analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
                && diagnostic.primary.message.contains("Int")
        }),
        "{:?}",
        analysis.diagnostics
    );
}
