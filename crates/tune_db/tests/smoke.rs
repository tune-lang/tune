#[test]
fn source_map_uses_diagnostic_file_ids() -> Result<(), &'static str> {
    let mut db = tune_db::TuneDb::new();

    let file = db
        .add_file("main.tn", "let x = 1")
        .ok_or("source map should allocate the first file")?;

    let span = tune_diagnostics::Span::new(
        file,
        tune_diagnostics::ByteOffset::new(0),
        tune_diagnostics::ByteOffset::new(3),
    );

    assert_eq!(file, tune_diagnostics::FileId(0));
    assert_eq!(span.file, file);
    assert_eq!(
        db.source(file).map(|source| source.path.as_str()),
        Some("main.tn")
    );
    assert_eq!(db.sources().len(), 1);

    Ok(())
}

#[test]
fn interner_uses_stable_symbol_ids() -> Result<(), &'static str> {
    let mut db = tune_db::TuneDb::new();

    let first = db.intern("value").ok_or("first symbol should allocate")?;
    let second = db.intern("value").ok_or("second symbol should resolve")?;
    let other = db.intern("other").ok_or("other symbol should allocate")?;

    assert_eq!(first, second);
    assert_ne!(first, other);
    assert_eq!(db.symbol(first), Some("value"));
    assert_eq!(db.symbol(other), Some("other"));
    assert_eq!(db.symbols().len(), 2);

    Ok(())
}

#[test]
fn analyzes_source_file_through_shared_frontend() -> Result<(), &'static str> {
    let mut db = tune_db::TuneDb::new();
    let file = db
        .add_file(
            "main.tn",
            r#"
tag tool {}
@tool
let run(input: String): String = input
"#,
        )
        .ok_or("source file should allocate")?;

    let analysis = db
        .analyze_file(file)
        .ok_or("analysis should find source file")?;

    assert!(analysis.diagnostics().is_empty());
    assert_eq!(analysis.module.items.len(), 2);
    assert!(analysis.resolved.scope.get("run").is_some());
    assert!(analysis.resolved.facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Tag(tag) if tag == "tool"
    )));

    Ok(())
}
