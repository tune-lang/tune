#[test]
fn engine_exposes_meta_decl_facts_from_shared_compiler_facts() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
-- Runs the app.
let run(): String = "ok"
"#,
        )
        .ok_or("source should allocate")?;
    let check = tune.check_file(file).ok_or("source should check")?;
    let run = check.module.items[0].id;

    let facts = tune
        .meta_decl_facts(file, run)
        .map_err(|_| "meta facts should resolve")?;

    assert!(
        facts.facts.iter().any(|fact| {
            matches!(fact, tune_meta::facts::DeclFact::Name(name) if name == "run")
        })
    );
    assert!(facts.facts.iter().any(|fact| {
        matches!(fact, tune_meta::facts::DeclFact::Doc(doc) if doc == "Runs the app.")
    }));
    assert!(
        facts
            .facts
            .iter()
            .any(|fact| matches!(fact, tune_meta::facts::DeclFact::Return(_)))
    );

    Ok(())
}

#[test]
fn engine_exposes_tagged_decls_without_tag_name_special_cases() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let file = tune
        .add_file(
            "app.tn",
            r#"
tag route {}
tag audit {}
@route(path = "/")
let home(): String = "home"
@audit(level = "debug")
let debug(): String = "debug"
"#,
        )
        .ok_or("source should allocate")?;

    let tagged = tune
        .meta_tagged(file, "audit")
        .map_err(|_| "tagged query should resolve")?;

    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].tag.name, "audit");
    assert_eq!(tagged[0].tag.args[0].name.as_deref(), Some("level"));

    Ok(())
}
