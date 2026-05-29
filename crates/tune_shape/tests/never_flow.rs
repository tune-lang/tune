fn analyze_item(source: &str, item: usize) -> tune_shape::ShapeAnalysis {
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    tune_shape::analyze_item(&module, &resolved, &module.items[item])
}

#[test]
fn user_never_returning_call_is_non_continuing_flow() {
    let source = r#"
let stop(): Never = panic("bad")
let result: Int = if true { stop() } else { 1 }
"#;
    let analysis = analyze_item(source, 1);

    assert!(analysis.diagnostics.is_empty());
    assert!(
        analysis
            .expr_shapes
            .iter()
            .any(|shape| shape.shape == tune_shape::Shape::Never)
    );
}
