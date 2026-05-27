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
