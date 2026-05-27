#[test]
fn reports_unresolved_body_names() {
    let source = r#"
let helper(value) = value
let run(input) = helper(input, missing)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert_eq!(resolved.diagnostics.len(), 1);
    assert_eq!(
        resolved.diagnostics[0].code,
        tune_diagnostics::codes::UNRESOLVED_NAME
    );
    assert_eq!(resolved.diagnostics[0].title, "unresolved name `missing`");
}

#[test]
fn reports_invalid_assignment_targets() {
    let source = "let run(a, b) = { a + b = 1 }";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert_eq!(resolved.diagnostics.len(), 1);
    assert_eq!(
        resolved.diagnostics[0].code,
        tune_diagnostics::codes::INVALID_ASSIGNMENT_TARGET
    );
}
