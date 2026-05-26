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
