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
fn records_type_param_facts() {
    let source = "enum Boxed<T> { Value(T) }";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::TypeParams(params) if params.len() == 1
    )));
    assert!(resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Name(name) if name == "T"
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
fn reports_compiler_reserved_user_names() {
    let source = r#"
let __doc = 1
let run(__name: String): String = __name
struct Box {
  __json_invoker: String
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert_eq!(
        resolved
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == tune_diagnostics::codes::COMPILER_RESERVED_NAME
            })
            .count(),
        3
    );
}

#[test]
fn reports_match_hole_fallback() {
    let source = r#"
let select(value) = match value {
  _ => value
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::MATCH_HOLE_FALLBACK
            && diagnostic.title == "`_` is a pattern hole, not a match fallback"
    }));
}

#[test]
fn records_tag_application_facts() {
    let source = r#"
tag tool {}
let capability = 1
@tool(capability = capability)
let run(input) = input
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.facts.iter().any(|fact| {
        fact.owner == tune_resolve::FactOwner::Item(tune_hir::HirId(2))
            && matches!(
                &fact.payload,
                tune_resolve::CompilerFactPayload::Tag(tag)
                    if tag.name == "tool"
                        && tag.args.len() == 1
                        && tag.args[0].name.as_deref() == Some("capability")
            )
            && fact.span.is_some()
    }));
    assert!(resolved.name_refs.iter().any(|name_ref| matches!(
        name_ref.target,
        tune_resolve::NameTarget::TopLevel(tune_hir::HirId(1))
    )));
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
let check(input, other) = not input and other is not none
let branch(input, ready) = if ready { helper(input) } else { panic("bad") }
let select(result) = match result { Ok(value) => helper(value); else => panic("bad") }
let repeated(ready) = while ready { continue }
let forever() = loop { break }
struct Box {
  value: String
  get(default: String): String = default
  [items] = items
  Box[index: Size]: String = index
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert!(
        resolved
            .locals
            .iter()
            .any(|local| { local.name == "local" && local.kind == tune_resolve::LocalKind::Let })
    );
    assert!(
        resolved.locals.iter().any(|local| {
            local.name == "item" && local.kind == tune_resolve::LocalKind::Pattern
        })
    );
    assert!(resolved.locals.iter().any(|local| {
        local.name == "x" && local.kind == tune_resolve::LocalKind::CallableParam
    }));
    assert!(resolved.name_refs.iter().any(|name_ref| matches!(
        name_ref.target,
        tune_resolve::NameTarget::Param(_)
            | tune_resolve::NameTarget::Local(_)
            | tune_resolve::NameTarget::TopLevel(_)
    )));
    assert!(resolved.variant_pattern_refs.iter().any(|variant_ref| {
        variant_ref.variant == tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Ok)
    }));
}

#[test]
fn resolves_prelude_result_variants_as_constructors_and_patterns() {
    let source = r#"
let ok(value) = Ok(value)
let fail(error) = Error(error)
let unwrap(result) = match result { Ok(value) => value; Error(error) => error }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.name_refs.iter().any(|name_ref| {
        name_ref.target
            == tune_resolve::NameTarget::Variant(tune_resolve::VariantId::Prelude(
                tune_resolve::PreludeVariant::Ok,
            ))
    }));
    assert!(resolved.name_refs.iter().any(|name_ref| {
        name_ref.target
            == tune_resolve::NameTarget::Variant(tune_resolve::VariantId::Prelude(
                tune_resolve::PreludeVariant::Error,
            ))
    }));
    assert_eq!(resolved.variant_pattern_refs.len(), 2);
}

#[test]
fn resolves_enum_variants_only_from_expected_shape_context() {
    let source = r#"
enum Color {
  Red
  Rgb(Int, Int, Int)
}
let inferred = Red
let annotated: Color = Rgb(1, 2, 3)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.name_refs.iter().any(|name_ref| matches!(
        name_ref.target,
        tune_resolve::NameTarget::Variant(tune_resolve::VariantId::Member(_))
    )));
    assert!(resolved.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::UNRESOLVED_NAME
            && diagnostic.title == "unresolved name `Red`"
    }));
}

#[test]
fn resolves_generic_enum_variants_from_expected_shape_context() {
    let source = r#"
enum Boxed<T> {
  Value(T)
}
let boxed: Boxed<String> = Value("hello")
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.name_refs.iter().any(|name_ref| matches!(
        name_ref.target,
        tune_resolve::NameTarget::Variant(tune_resolve::VariantId::Member(_))
    )));
}
