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

    Ok(())
}
