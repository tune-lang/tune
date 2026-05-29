#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn meta_facts_use_compiler_ids_and_shapes() {
    let facts = tune_meta::facts::DeclFacts {
        decl_id: tune_hir::HirId(7),
        facts: vec![
            tune_meta::facts::DeclFact::Name("run".into()),
            tune_meta::facts::DeclFact::Return(tune_shape::Shape::Task(Box::new(
                tune_shape::Shape::Unit,
            ))),
            tune_meta::facts::DeclFact::Visibility(tune_hir::item::Visibility::Public),
        ],
    };
    let tagged = tune_meta::tagged::TaggedDecl {
        tag: "tool",
        decl_id: facts.decl_id,
    };

    assert_eq!(tagged.decl_id, tune_hir::HirId(7));
    assert!(matches!(
        facts.facts[1],
        tune_meta::facts::DeclFact::Return(tune_shape::Shape::Task(_))
    ));
}

#[test]
fn json_invoker_is_a_compiler_generated_plan() {
    let invoker = tune_meta::json_invoker::generate_json_invoker(tune_hir::HirId(9));

    assert_eq!(invoker.decl_id, tune_hir::HirId(9));
    assert_eq!(invoker.helper_name, "__json_invoker_9");
    assert!(!invoker.uses_runtime_reflection);
}

#[test]
fn meta_decl_facts_are_derived_from_compiler_facts() {
    let source = r#"
tag tool {}
@tool
let run(): String = "ok"
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let facts = tune_meta::facts::from_compiler_facts(module.items[1].id, &resolved.facts);

    assert!(facts.facts.iter().any(|fact| {
        matches!(fact, tune_meta::facts::DeclFact::Name(name) if name == "run")
    }));
    assert!(facts
        .facts
        .iter()
        .any(|fact| matches!(fact, tune_meta::facts::DeclFact::JsonInvoker)));
}
