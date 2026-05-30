#[test]
fn lsp_session_queries_shared_db_diagnostics_and_facts() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let file = session
        .add_file(
            "main.tn",
            r#"
-- Run docs.
let run(input: String): String = input
"#,
        )
        .ok_or("source file should allocate")?;

    assert!(session.diagnostics(file).is_empty());

    let facts = session.facts_for_owner(file, tune_resolve::FactOwner::Item(tune_hir::HirId(0)));
    assert!(facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Name(name) if name == "run"
    )));
    assert!(facts.iter().any(|fact| matches!(
        &fact.payload,
        tune_resolve::CompilerFactPayload::Doc(doc) if doc == "Run docs."
    )));

    let hover = session
        .hover_card(file, tune_resolve::FactOwner::Item(tune_hir::HirId(0)))
        .ok_or("item facts should build a hover card")?;
    assert_eq!(hover.documentation.as_deref(), Some("Run docs."));
    assert_eq!(
        hover.signature.as_deref(),
        Some("let run(input: String): String")
    );
    let markdown = hover.markdown();
    assert!(markdown.contains("Run docs."));
    assert!(markdown.contains("```tn"));
    assert!(markdown.contains("let run(input: String): String"));

    Ok(())
}

#[test]
fn lsp_session_adapts_compiler_diagnostics() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let file = session
        .add_file("main.tn", "let count: Int = \"not an int\"\n")
        .ok_or("source file should allocate")?;

    let diagnostics = session.lsp_diagnostics(file);
    let first = diagnostics
        .first()
        .ok_or("shape mismatch should produce an LSP diagnostic")?;
    assert_eq!(first.severity, tune_lsp::DiagnosticSeverity::Error);
    assert!(!first.code.is_empty());
    assert!(!first.message.is_empty());

    let hovers = session.diagnostic_hovers(file);
    let hover = hovers
        .first()
        .ok_or("shape mismatch should produce a diagnostic hover")?;
    assert_eq!(hover.diagnostic, *first);
    assert!(hover.markdown.contains("error["));
    assert!(!hover.markdown.contains("-->"));

    Ok(())
}

#[test]
fn lsp_ranges_use_utf16_positions() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let file = session
        .add_file("main.tn", "let face = \"😀\"\nlet next = 1\n")
        .ok_or("source file should allocate")?;
    let source = session.db().source(file).ok_or("source should exist")?;
    let start = source
        .text
        .find("next")
        .ok_or("fixture should contain target text")?;
    let end = start + "next".len();
    let span = tune_diagnostics::Span::new(
        file,
        tune_diagnostics::ByteOffset::new(start.try_into().map_err(|_| "start fits")?),
        tune_diagnostics::ByteOffset::new(end.try_into().map_err(|_| "end fits")?),
    );

    let range = tune_lsp::protocol::range(session.db(), span).ok_or("span should map")?;
    assert_eq!(
        range.start,
        tune_lsp::Position {
            line: 1,
            character: 4
        }
    );
    assert_eq!(
        range.end,
        tune_lsp::Position {
            line: 1,
            character: 8
        }
    );

    Ok(())
}
