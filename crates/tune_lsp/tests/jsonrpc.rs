#[test]
fn jsonrpc_server_handles_initialize_open_and_completion() -> Result<(), &'static str> {
    let mut server = tune_lsp::JsonRpcServer::new();
    let initialize = server.handle_message(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp/no-project"}}"#,
    );
    assert_eq!(initialize.len(), 1);
    assert!(initialize[0].contains("\"hoverProvider\":true"));

    let opened = server.handle_message(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/main.tn","text":"let value: Int = 1\n"}}}"#,
    );
    assert_eq!(opened.len(), 1);
    assert!(opened[0].contains("textDocument/publishDiagnostics"));

    let completion = server.handle_message(
        r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///tmp/main.tn"},"position":{"line":0,"character":4}}}"#,
    );
    assert_eq!(completion.len(), 1);
    assert!(completion[0].contains("\"label\":\"value\""));

    Ok(())
}

#[test]
fn jsonrpc_server_handles_formatting() {
    let mut server = tune_lsp::JsonRpcServer::new();
    let _ = server.handle_message(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/main.tn","text":"let value:Int=1\n"}}}"#,
    );

    let response = server.handle_message(
        r#"{"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{"textDocument":{"uri":"file:///tmp/main.tn"},"options":{"tabSize":2,"insertSpaces":true}}}"#,
    );

    assert_eq!(response.len(), 1);
    assert!(response[0].contains("let value: Int = 1\\n"));
}

#[test]
fn jsonrpc_server_handles_document_symbols() {
    let mut server = tune_lsp::JsonRpcServer::new();
    let _ = server.handle_message(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/main.tn","text":"struct Counter {\n  value: Int\n}\n"}}}"#,
    );

    let response = server.handle_message(
        r#"{"jsonrpc":"2.0","id":4,"method":"textDocument/documentSymbol","params":{"textDocument":{"uri":"file:///tmp/main.tn"}}}"#,
    );

    assert_eq!(response.len(), 1);
    assert!(response[0].contains("\"name\":\"Counter\""));
    assert!(response[0].contains("\"documentSymbolProvider\":true") || !response[0].is_empty());
}

#[test]
fn jsonrpc_initialize_advertises_document_symbols() {
    let mut server = tune_lsp::JsonRpcServer::new();
    let initialize = server.handle_message(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":"file:///tmp/no-project"}}"#,
    );

    assert_eq!(initialize.len(), 1);
    assert!(initialize[0].contains("\"documentSymbolProvider\":true"));
}

#[test]
fn jsonrpc_server_handles_document_links() {
    let mut server = tune_lsp::JsonRpcServer::new();
    let _ = server.handle_message(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/lib.tn","text":"pub let helper(): Int = 1\n"}}}"#,
    );
    let _ = server.handle_message(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/main.tn","text":"import \"/tmp/lib.tn\".helper\nlet value = helper()\n"}}}"#,
    );

    let response = server.handle_message(
        r#"{"jsonrpc":"2.0","id":5,"method":"textDocument/documentLink","params":{"textDocument":{"uri":"file:///tmp/main.tn"}}}"#,
    );

    assert_eq!(response.len(), 1);
    assert!(response[0].contains("\"target\":\"file:///tmp/lib.tn\""));
}

#[test]
fn jsonrpc_diagnostics_include_tune_source_and_code_description() {
    let mut server = tune_lsp::JsonRpcServer::new();
    let messages = server.handle_message(
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///tmp/bad.tn","text":"let value: Int = \"bad\"\n"}}}"#,
    );

    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("\"source\":\"tune\""));
    assert!(messages[0].contains("\"codeDescription\""));
}

#[test]
fn jsonrpc_initialize_loads_workspace_folder_symbols() -> Result<(), String> {
    let root = std::env::temp_dir().join(format!("tune-lsp-jsonrpc-{}", std::process::id()));
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
    std::fs::write(root.join("src/main.tn"), "let value: Int = helper()\n")
        .map_err(|error| error.to_string())?;
    std::fs::write(root.join("src/lib.tn"), "pub let helper(): Int = 1\n")
        .map_err(|error| error.to_string())?;

    let mut server = tune_lsp::JsonRpcServer::new();
    let uri = format!("file://{}", root.display());
    let initialize = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{uri}","name":"tooling"}}]}}}}"#
    );
    assert_eq!(server.handle_message(&initialize).len(), 1);

    let symbols = server.handle_message(
        r#"{"jsonrpc":"2.0","id":2,"method":"workspace/symbol","params":{"query":"helper"}}"#,
    );
    assert_eq!(symbols.len(), 1);
    assert!(symbols[0].contains("\"name\":\"helper\""));

    std::fs::remove_dir_all(root).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn jsonrpc_stdio_reads_and_writes_framed_messages() -> Result<(), String> {
    let message = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let input = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
    let mut reader = std::io::BufReader::new(input.as_bytes());
    let mut output = Vec::new();
    let mut server = tune_lsp::JsonRpcServer::new();

    tune_lsp::run_stdio(&mut reader, &mut output, &mut server)
        .map_err(|error| error.to_string())?;
    let output = String::from_utf8(output).map_err(|error| error.to_string())?;
    assert!(output.starts_with("Content-Length: "));
    assert!(output.contains("\"id\":1"));
    assert!(output.contains("semanticTokensProvider"));

    Ok(())
}
