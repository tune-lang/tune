fn analyze(source: &str) -> tune_shape::ShapeAnalysis {
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    tune_shape::analyze_item(&module, &resolved, &module.items[0])
}

#[test]
fn analyzer_reports_compile_time_int_overflow() {
    let analysis = analyze("let result: Int = 9223372036854775807 + 1");

    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == tune_diagnostics::codes::NUMERIC_OVERFLOW),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn analyzer_reports_compile_time_size_underflow() {
    let analysis = analyze("let result: Size = 0 - 1");

    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == tune_diagnostics::codes::NUMERIC_OVERFLOW),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn analyzer_allows_byte_compile_time_wrapping_arithmetic() {
    let analysis = analyze("let result: Byte = 255 + 1");

    assert!(
        analysis
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != tune_diagnostics::codes::NUMERIC_OVERFLOW),
        "{:?}",
        analysis.diagnostics
    );
}
