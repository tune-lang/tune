#[test]
fn analyzer_checks_top_level_call_signatures() -> Result<(), &'static str> {
    let source = r#"
let add(value: Int): Int = value
let run(): Int = add("bad")
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.calls.iter().any(|call| {
        matches!(
            call.target,
            tune_shape::CallTarget::TopLevel(tune_hir::HirId(0))
        ) && call.params == vec![tune_shape::Shape::Int]
            && call.ret == tune_shape::Shape::Int
    }));
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "call argument does not match callable parameter shape"
    }));

    Ok(())
}

#[test]
fn analyzer_checks_member_call_signatures_with_receiver() -> Result<(), &'static str> {
    let source = r#"
struct Counter {
  inc(value: Int): Int = value
}
let run(counter: Counter): Int = counter.inc("bad")
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.calls.iter().any(|call| {
        matches!(call.target, tune_shape::CallTarget::Member(_))
            && call.params == vec![tune_shape::Shape::Int]
            && call.ret == tune_shape::Shape::Int
            && call.receiver == Some(tune_shape::Shape::Struct("Counter".to_owned()))
    }));
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH })
    );

    Ok(())
}

#[test]
fn analyzer_checks_enum_constructor_payloads() -> Result<(), &'static str> {
    let source = r#"
enum Color {
  Rgb(Int, Int, Int)
}
let make: Color = Rgb(1, "bad", 3)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.calls.iter().any(|call| {
        matches!(call.target, tune_shape::CallTarget::Variant(_))
            && call.params
                == vec![
                    tune_shape::Shape::Int,
                    tune_shape::Shape::Int,
                    tune_shape::Shape::Int,
                ]
    }));
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "call argument does not match callable parameter shape"
    }));

    Ok(())
}

#[test]
fn analyzer_checks_enum_constructor_arity() -> Result<(), &'static str> {
    let source = r#"
enum Color {
  Rgb(Int, Int, Int)
}
let make: Color = Rgb(1)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "call argument count does not match callable signature"
    }));

    Ok(())
}
