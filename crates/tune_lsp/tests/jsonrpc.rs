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
