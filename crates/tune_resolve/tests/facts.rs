#[test]
fn records_callable_member_signature_facts_with_stable_param_ids() {
    let source = r#"
struct Counter {
  add(value: Int): Int = value
  reset(value: String): String = value
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let callable_ids = module.items[0]
        .struct_members
        .iter()
        .filter_map(|member| match member {
            tune_hir::item::StructMember::Callable(callable) => Some(callable.id),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(callable_ids.len(), 2);
    assert!(callable_ids.iter().all(|id| {
        resolved.facts.iter().any(|fact| {
            fact.owner == tune_resolve::FactOwner::Member(*id)
                && fact.kind() == tune_resolve::CompilerFactKind::Params
        }) && resolved.facts.iter().any(|fact| {
            fact.owner == tune_resolve::FactOwner::Member(*id)
                && fact.kind() == tune_resolve::CompilerFactKind::Return
        })
    }));

    let param_ids = module.items[0]
        .struct_members
        .iter()
        .flat_map(|member| match member {
            tune_hir::item::StructMember::Callable(callable) => callable
                .params
                .iter()
                .map(|param| param.id)
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();

    assert_eq!(param_ids.len(), 2);
    assert_ne!(param_ids[0], param_ids[1]);
}
