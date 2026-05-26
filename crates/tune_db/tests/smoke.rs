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
