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
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Name(name) if name == "run"
    )));
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Doc(doc) if doc == "Run docs."
    )));
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Visibility(tune_hir::item::Visibility::Private)
    )));
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

#[test]
fn records_callable_signature_facts() {
    let source = "let parse(text: String, strict: Bool): Result = text";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Params(params) if params.len() == 2
    )));
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Name(name) if name == "text"
    )));
    assert!(
        resolved
            .facts
            .iter()
            .any(|fact| fact.kind() == tune_resolve::CompilerFactKind::Return)
    );
}

#[test]
fn records_member_surface_facts() {
    let source = r#"
struct User {
  name: String
  age: Int
}
enum LoadResult {
  Ok(User)
  Error(String)
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Fields(fields) if fields.len() == 2
    )));
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Variants(variants) if variants.len() == 2
    )));
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Name(name) if name == "age"
    )));
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Name(name) if name == "Error"
    )));
}

#[test]
fn reports_duplicate_member_names() {
    let source = r#"
let parse(text: String, text: String): String = text
struct User {
  name: String
  name: String
}
enum LoadResult {
  Ok(User)
  Ok(String)
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert_eq!(resolved.diagnostics.len(), 3);
    assert!(
        resolved
            .diagnostics
            .iter()
            .all(|diagnostic| { diagnostic.code == tune_diagnostics::codes::DUPLICATE_NAME })
    );
    assert!(
        resolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.title == "duplicate parameter `text`")
    );
    assert!(
        resolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.title == "duplicate field `name`")
    );
    assert!(
        resolved
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.title == "duplicate variant `Ok`")
    );
}

#[test]
fn records_tag_application_facts() {
    let source = r#"
tag tool {}
@tool
let run(input) = input
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.facts.iter().any(|fact| {
        fact.owner == tune_resolve::FactOwner::Item(tune_hir::HirId(1))
            && matches!(
                &fact.payload,
                tune_resolve::CompilerFactPayload::Tag(tag) if tag == "tool"
            )
            && fact.span.is_some()
    }));
}

#[test]
fn reports_unresolved_tag_applications() {
    let source = r#"
@missing
let run(input) = input
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert_eq!(resolved.diagnostics.len(), 1);
    assert_eq!(
        resolved.diagnostics[0].code,
        tune_diagnostics::codes::UNRESOLVED_NAME
    );
    assert_eq!(resolved.diagnostics[0].title, "unresolved tag `missing`");
}

#[test]
fn resolves_body_names_from_items_params_and_for_patterns() {
    let source = r#"
let helper(value) = value
let run(input) = helper(input)
let each(items) = for item in items { helper(item) }
let scoped(input) = { let local = _(x) = helper(x); local(input) }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
}

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
