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
    assert!(
        session
            .workspace_symbols("helper")
            .iter()
            .any(|symbol| { symbol.name == "helper" && symbol.path == "src/lib.tn" })
    );

    std::fs::remove_dir_all(root).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn lsp_open_buffers_override_project_loaded_sources() -> Result<(), String> {
    let root =
        std::env::temp_dir().join(format!("tune-lsp-project-override-{}", std::process::id()));
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
    std::fs::write(root.join("src/main.tn"), "let value: Int = 1\n")
        .map_err(|error| error.to_string())?;

    let mut session = tune_lsp::LspSession::new();
    session
        .open_project_dir(&root)
        .map_err(|error| format!("{error:?}"))?;
    let project_file = session
        .file_for_path("src/main.tn")
        .ok_or_else(|| "project file should be loaded".to_owned())?;

    let opened = session
        .open_document(
            root.join("src/main.tn").to_string_lossy().into_owned(),
            "let value: Int = \"bad\"\n",
        )
        .ok_or_else(|| "absolute open should map to project file".to_owned())?;
    assert_eq!(opened, project_file);
    assert!(!session.diagnostics(project_file).is_empty());

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

#[test]
fn lsp_code_actions_skip_already_imported_names() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let main = session
        .open_document(
            "src/main.tn",
            "import \"src/lib.tn\".helper\nlet value: Int = helper()\n",
        )
        .ok_or("main should open")?;
    session
        .open_document("src/lib.tn", "pub let helper(): Int = 1\n")
        .ok_or("lib should open")?;

    assert!(
        session
            .code_actions(main)
            .iter()
            .all(|action| !action.title.contains("Import `helper`"))
    );

    Ok(())
}

#[test]
fn lsp_formatting_uses_shared_formatter() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let file = session
        .open_document("main.tn", "let value:Int=1\n")
        .ok_or("document should open")?;

    let edits = session.formatting(file);
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].replacement, "let value: Int = 1\n");

    Ok(())
}

#[test]
fn lsp_completion_filters_visible_names_by_prefix() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let source = "let apple: Int = 1\nlet banana: Int = 2\nlet result = ap\n";
    let file = session
        .open_document("main.tn", source)
        .ok_or("document should open")?;
    let offset = source.find("ap\n").ok_or("cursor marker should exist")? + "ap".len();
    let position =
        tune_lsp::protocol::position(source, u32::try_from(offset).map_err(|_| "bad offset")?)
            .ok_or("position should map")?;

    let completions = session.completions_at(file, position);
    assert!(completions.iter().any(|item| item.label == "apple"));
    assert!(completions.iter().all(|item| item.label != "banana"));

    Ok(())
}

#[test]
fn lsp_completion_suggests_struct_members_after_dot() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let source = r#"struct Counter {
  value: Int

  bump(amount: Int): Int = self.value + amount
}

let counter: Counter = Counter {
  value = 1
}
let result = counter.b
"#;
    let file = session
        .open_document("main.tn", source)
        .ok_or("document should open")?;
    let offset = source
        .find("counter.b")
        .ok_or("cursor marker should exist")?
        + "counter.b".len();
    let position =
        tune_lsp::protocol::position(source, u32::try_from(offset).map_err(|_| "bad offset")?)
            .ok_or("position should map")?;

    let completions = session.completions_at(file, position);
    assert!(completions.iter().any(|item| {
        item.label == "bump"
            && item.kind == tune_lsp::CompletionKind::Method
            && item.detail.as_deref() == Some("bump(amount: Int): Int")
    }));
    assert!(completions.iter().all(|item| item.label != "value"));

    Ok(())
}

#[test]
fn lsp_semantic_tokens_distinguish_struct_members() -> Result<(), &'static str> {
    let mut session = tune_lsp::LspSession::new();
    let source = r#"struct Counter {
  value: Int

  bump(): Int = self.value
}

let counter: Counter = Counter {
  value = 1
}
let result = counter.bump()
"#;
    let file = session
        .open_document("main.tn", source)
        .ok_or("document should open")?;

    let tokens = session.semantic_tokens(file);
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == tune_lsp::SemanticTokenKind::Property)
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == tune_lsp::SemanticTokenKind::Function)
    );

    Ok(())
}
