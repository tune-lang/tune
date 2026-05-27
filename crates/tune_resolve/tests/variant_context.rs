fn resolve_source(source: &str) -> tune_resolve::ResolvedModule {
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    tune_resolve::resolve_module(&module)
}

#[test]
fn resolves_user_enum_patterns_from_param_shape_context() {
    let resolved = resolve_source(
        r#"
enum Choice {
  One(Int)
  Two(Int)
}
let pick(choice: Choice): Int = match choice {
  One(value) => value
  Two(value) => value
}
"#,
    );

    assert!(resolved.diagnostics.is_empty());
    assert_eq!(resolved.variant_pattern_refs.len(), 2);
    assert!(
        resolved
            .variant_pattern_refs
            .iter()
            .all(|variant_ref| matches!(variant_ref.variant, tune_resolve::VariantId::Member(_)))
    );
}

#[test]
fn resolves_user_enum_patterns_from_local_shape_context() {
    let resolved = resolve_source(
        r#"
enum Choice {
  One(Int)
  Two(Int)
}
let pick(): Int = {
  let choice: Choice = One(2)
  match choice {
    One(value) => value
    Two(value) => value
  }
}
"#,
    );

    assert!(resolved.diagnostics.is_empty());
    assert_eq!(resolved.variant_pattern_refs.len(), 2);
    assert!(
        resolved
            .variant_pattern_refs
            .iter()
            .all(|variant_ref| matches!(variant_ref.variant, tune_resolve::VariantId::Member(_)))
    );
}

#[test]
fn resolves_user_enum_constructors_from_call_arg_context() {
    let resolved = resolve_source(
        r#"
enum Choice {
  One(Int)
  Two(Int)
}
let pick(choice: Choice): Int = 1
let result: Int = pick(Two(4))
"#,
    );

    assert!(resolved.diagnostics.is_empty());
    assert!(resolved.name_refs.iter().any(|name_ref| matches!(
        name_ref.target,
        tune_resolve::NameTarget::Variant(tune_resolve::VariantId::Member(_))
    )));
}

#[test]
fn leaves_user_enum_patterns_unresolved_without_shape_context() {
    let resolved = resolve_source(
        r#"
enum Choice {
  One(Int)
}
let pick(value) = match value {
  One(inner) => inner
}
"#,
    );

    assert!(resolved.variant_pattern_refs.is_empty());
    assert!(resolved.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::UNRESOLVED_NAME
            && diagnostic.title == "unresolved variant pattern `One`"
    }));
}
