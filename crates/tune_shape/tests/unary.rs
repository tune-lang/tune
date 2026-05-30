#[test]
fn not_and_tilde_are_the_same_inversion_operator() {
    let source = r#"
let bool_value: Bool = not false
let int_value: Int = not 1
let byte_value: Byte = ~1
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_module(&module, &resolved);

    assert!(analysis.iter().all(|item| item.diagnostics.is_empty()));
    assert!(matches!(
        analysis[0].expr_shapes.last().map(|shape| &shape.shape),
        Some(tune_shape::Shape::Bool)
    ));
    assert!(matches!(
        analysis[1].expr_shapes.last().map(|shape| &shape.shape),
        Some(tune_shape::Shape::Int)
    ));
    assert!(matches!(
        analysis[2].expr_shapes.last().map(|shape| &shape.shape),
        Some(tune_shape::Shape::Byte)
    ));
}
