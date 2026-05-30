#[test]
fn analyzer_narrows_optional_payload_after_none_check() -> Result<(), &'static str> {
    let source = r#"
let add_one(value: Int?): Int = if value is not none {
  value + 1
} else {
  0
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(
        analysis.diagnostics.is_empty(),
        "optional payload should narrow to Int inside the not-none branch: {:?}",
        analysis.diagnostics
    );

    Ok(())
}

#[test]
fn analyzer_narrows_optional_payload_in_none_check_else() -> Result<(), &'static str> {
    let source = r#"
let add_one(value: Int?): Int = if value is none {
  0
} else {
  value + 1
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(
        analysis.diagnostics.is_empty(),
        "optional payload should narrow to Int inside the else branch: {:?}",
        analysis.diagnostics
    );

    Ok(())
}

#[test]
fn analyzer_narrows_optional_payload_after_guard_return() -> Result<(), &'static str> {
    let source = r#"
let add_one(value: Int?): Int = {
  if value is none {
    return 0
  }
  value + 1
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(
        analysis.diagnostics.is_empty(),
        "optional payload should narrow after the returning guard branch: {:?}",
        analysis.diagnostics
    );

    Ok(())
}

#[test]
fn analyzer_narrows_optional_payload_through_short_circuit_condition() -> Result<(), &'static str> {
    let source = r#"
let positive(value: Int?): Int = if value is not none and value > 0 {
  value + 1
} else {
  0
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(
        analysis.diagnostics.is_empty(),
        "optional payload should narrow for RHS and body of short-circuit condition: {:?}",
        analysis.diagnostics
    );

    Ok(())
}

#[test]
fn analyzer_does_not_warn_when_optional_is_proven_present() -> Result<(), &'static str> {
    let analysis = analyze_last_item(
        r#"
let x: Int? = 0
let y = x
"#,
    )?;

    assert!(
        analysis.diagnostics.is_empty(),
        "copying a proven-present optional should not warn: {:?}",
        analysis.diagnostics
    );

    Ok(())
}

#[test]
fn analyzer_errors_when_optional_is_proven_none() -> Result<(), &'static str> {
    let analysis = analyze_last_item(
        r#"
let x: Int?
let y = x
"#,
    )?;

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == tune_diagnostics::Severity::Error
            && diagnostic.title == "optional value is proven none"
    }));

    Ok(())
}

#[test]
fn analyzer_warns_when_optional_may_be_none() -> Result<(), &'static str> {
    let analysis = analyze_last_item(
        r#"
let maybe(): Int? = none
let x: Int? = maybe()
let y = x
"#,
    )?;

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == tune_diagnostics::Severity::Warning
            && diagnostic.title == "optional value may be none"
    }));

    Ok(())
}

fn analyze_last_item(source: &str) -> Result<tune_shape::ShapeAnalysis, &'static str> {
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let item = module.items.last().ok_or("expected at least one item")?;
    Ok(tune_shape::analyze_item(&module, &resolved, item))
}
