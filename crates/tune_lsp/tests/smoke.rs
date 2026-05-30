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
    let source = session.db().source(file).ok_or("source should exist")?;
    let run_offset = source
        .text
        .find("run")
        .ok_or("fixture should contain callable name")?;
    let run_span = tune_diagnostics::Span::new(
        file,
        tune_diagnostics::ByteOffset::new(run_offset.try_into().map_err(|_| "start fits")?),
        tune_diagnostics::ByteOffset::new((run_offset + 1).try_into().map_err(|_| "end fits")?),
    );
    let run_position = tune_lsp::protocol::range(session.db(), run_span)
        .ok_or("callable span should map")?
        .start;
    let position_hover = session
        .hover_card_at(file, run_position)
        .ok_or("callable position should map to a hover card")?;
    assert_eq!(position_hover.signature, hover.signature);

    let completions = session.completions(file);
    let run = completions
        .iter()
        .find(|item| item.label == "run")
        .ok_or("callable should be offered as a completion")?;
    assert_eq!(run.kind, tune_lsp::CompletionKind::Function);
    assert_eq!(run.documentation.as_deref(), Some("Run docs."));
    assert_eq!(
        run.detail.as_deref(),
        Some("let run(input: String): String")
    );

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

#[test]
fn lsp_session_uses_cursor_facts_for_expression_tooling() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let source = r#"
-- Adds two ints.
let add(a: Int, b: Int): Int = a + b
let value: Int = add(1, 2)
"#;
    let file = session
        .add_file("main.tn", source)
        .ok_or("source file should allocate")?;

    let source_file = session.db().source(file).ok_or("source should exist")?;
    let two_offset = source_file
        .text
        .find("2)")
        .ok_or("fixture should contain second argument")?;
    let two_position = tune_lsp::protocol::position(
        &source_file.text,
        two_offset.try_into().map_err(|_| "offset fits")?,
    )
    .ok_or("argument offset should map to position")?;

    let signature = session
        .signature_help_at(file, two_position)
        .ok_or("signature help should resolve at argument")?;
    assert_eq!(signature.active_parameter, Some(1));
    assert_eq!(signature.signature, "add(arg0: Int, arg1: Int): Int");

    let hover = session
        .hover_card_at(file, two_position)
        .ok_or("expression hover should show inferred shape")?;
    assert!(hover.markdown().contains("inferred shape Int"));

    let call_offset = source_file
        .text
        .find("add(1")
        .ok_or("fixture should contain call")?;
    let call_position = tune_lsp::protocol::position(
        &source_file.text,
        call_offset.try_into().map_err(|_| "offset fits")?,
    )
    .ok_or("call offset should map to position")?;
    let definition = session
        .definition_at(file, call_position)
        .ok_or("call target should resolve definition")?;
    assert_eq!(definition.name.as_deref(), Some("add"));

    Ok(())
}

#[test]
fn lsp_session_uses_cursor_facts_for_scoped_completion_and_references() -> Result<(), &'static str>
{
    let mut session = tune_lsp::LspSession::new();
    let source = r#"
let outer: Int = 1
let run(input: Int): Int = {
  let local: Int = input
  local
}
"#;
    let file = session
        .add_file("main.tn", source)
        .ok_or("source file should allocate")?;
    let source_file = session.db().source(file).ok_or("source should exist")?;
    let final_local_offset = source_file
        .text
        .rfind("local")
        .ok_or("fixture should contain local reference")?;
    let final_local_position = tune_lsp::protocol::position(
        &source_file.text,
        final_local_offset.try_into().map_err(|_| "offset fits")?,
    )
    .ok_or("local offset should map to position")?;

    let completions = session.completions_at(file, final_local_position);
    assert!(completions.iter().any(|item| item.label == "outer"));
    assert!(completions.iter().any(|item| item.label == "input"));
    assert!(completions.iter().any(|item| item.label == "local"));

    let definition = session
        .definition_at(file, final_local_position)
        .ok_or("local reference should resolve definition")?;
    assert_eq!(definition.name.as_deref(), Some("local"));

    let references = session.references_at(file, final_local_position);
    assert_eq!(references.len(), 2);

    Ok(())
}

#[test]
fn lsp_session_updates_documents_and_dispatches_requests() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let file = session
        .open_document("main.tn", "let value: Int = 1\n")
        .ok_or("document should open")?;
    assert_eq!(session.file_for_path("main.tn"), Some(file));
    assert!(session.diagnostics(file).is_empty());

    let changed = session
        .change_document("main.tn", "let value: Int = \"bad\"\n")
        .ok_or("document should change")?;
    assert_eq!(changed, file);
    assert!(!session.diagnostics(file).is_empty());

    let response = session.handle_request(tune_lsp::LspRequest::Completion {
        file,
        position: tune_lsp::Position {
            line: 0,
            character: 4,
        },
    });
    let tune_lsp::LspResponse::Completion(items) = response else {
        return Err("completion request should return completions");
    };
    assert!(items.iter().any(|item| item.label == "value"));

    assert_eq!(session.close_document("main.tn"), Some(file));
    Ok(())
}

#[test]
fn lsp_session_renames_resolved_references() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let source = r#"
let run(input: Int): Int = {
  let local: Int = input
  local
}
"#;
    let file = session
        .open_document("main.tn", source)
        .ok_or("document should open")?;
    let source_file = session.db().source(file).ok_or("source should exist")?;
    let final_local_offset = source_file
        .text
        .rfind("local")
        .ok_or("fixture should contain local reference")?;
    let position = tune_lsp::protocol::position(
        &source_file.text,
        final_local_offset.try_into().map_err(|_| "offset fits")?,
    )
    .ok_or("local offset should map")?;

    let edit = session
        .rename_at(file, position, "renamed")
        .ok_or("rename should produce workspace edit")?;
    assert_eq!(edit.file, file);
    assert_eq!(edit.edits.len(), 2);
    assert!(edit.edits.iter().all(|edit| edit.replacement == "renamed"));
    assert!(session.rename_at(file, position, "__bad").is_none());

    Ok(())
}

#[test]
fn lsp_session_reports_inlays_semantic_tokens_and_code_actions() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let source = r#"
pub let add(a, b) = a + b
let value = add(1, 2)
"#;
    let file = session
        .open_document("main.tn", source)
        .ok_or("document should open")?;

    let inlays = session.inlay_hints(file);
    assert!(inlays.iter().any(|hint| hint.label.contains("Int")));

    let tokens = session.semantic_tokens(file);
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == tune_lsp::SemanticTokenKind::Function)
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == tune_lsp::SemanticTokenKind::Parameter)
    );

    let actions = session.code_actions(file);
    let action = actions
        .iter()
        .find(|action| action.title == "Insert inferred public signature")
        .ok_or("public inference warning should produce code action")?;
    let edit = action.edit.as_ref().ok_or("code action should have edit")?;
    assert_eq!(edit.file, file);
    assert!(edit.edits.iter().any(
        |edit| edit.replacement.starts_with("pub let add(") && edit.replacement.contains("):")
    ));

    Ok(())
}

#[test]
fn lsp_session_loads_project_sources_for_tooling() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("tune-lsp-project-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(root.join("src")).map_err(|error| error.to_string())?;
    std::fs::write(
        root.join("dyno.toml"),
        r#"[project]
name = "tooling"
entry = "src/main.tn"
"#,
    )
    .map_err(|error| error.to_string())?;
    std::fs::write(root.join("src/main.tn"), "let value = helper()\n")
        .map_err(|error| error.to_string())?;
    std::fs::write(root.join("src/lib.tn"), "pub let helper(): Int = 1\n")
        .map_err(|error| error.to_string())?;

    let mut session = tune_lsp::LspSession::new();
    let files = session
        .open_project_dir(&root)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(files.len(), 2);
    assert!(session.file_for_path("src/main.tn").is_some());
    assert!(session.file_for_path("src/lib.tn").is_some());

    std::fs::remove_dir_all(root).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn lsp_code_actions_suggest_imports_from_loaded_public_items() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let main = session
        .open_document(
            "src/main.tn",
            "-- Main module.\n\nlet value: Int = helper()\n",
        )
        .ok_or("main should open")?;
    session
        .open_document("src/lib.tn", "pub let helper(): Int = 1\n")
        .ok_or("lib should open")?;

    let actions = session.code_actions(main);
    let action = actions
        .iter()
        .find(|action| action.title == "Import `helper` from \"src/lib.tn\"")
        .ok_or("unresolved helper should produce an import action")?;
    let edit = action.edit.as_ref().ok_or("import action should edit")?;
    assert_eq!(edit.file, main);
    assert_eq!(edit.edits.len(), 1);
    assert_eq!(edit.edits[0].replacement, "import \"src/lib.tn\".helper\n");
    assert_eq!(
        edit.edits[0].range.start,
        tune_lsp::Position {
            line: 2,
            character: 0
        }
    );

    Ok(())
}
