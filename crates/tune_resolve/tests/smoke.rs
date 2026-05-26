#[test]
fn resolves_top_level_bindings() {
    let source = r#"
import "std/json"
tag tool {}
-- Run docs.
let run(input) = input
struct Counter {}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert_eq!(resolved.scope.len(), 4);
    assert!(matches!(
        resolved.scope.get("run").map(|binding| binding.kind),
        Some(tune_resolve::BindingKind::StableCallableDecl)
    ));
    assert!(matches!(
        resolved.scope.get("Counter").map(|binding| binding.kind),
        Some(tune_resolve::BindingKind::Struct)
    ));
    assert!(
        resolved.facts.iter().any(|fact| {
            fact.kind == tune_resolve::CompilerFactKind::Name && fact.value == "run"
        })
    );
    assert!(resolved.facts.iter().any(|fact| {
        fact.kind == tune_resolve::CompilerFactKind::Doc && fact.value == "Run docs."
    }));
    assert!(resolved.facts.iter().any(|fact| {
        fact.kind == tune_resolve::CompilerFactKind::Visibility && fact.value == "private"
    }));
}

#[test]
fn reports_duplicate_top_level_bindings() {
    let source = "let value = 1\nlet value = 2";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert_eq!(resolved.diagnostics.len(), 1);
    assert_eq!(
        resolved.diagnostics[0].code,
        tune_diagnostics::codes::DUPLICATE_NAME
    );
}
