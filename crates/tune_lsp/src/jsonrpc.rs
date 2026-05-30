use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

use crate::server::LspSession;

mod value;
use value::*;

#[derive(Default)]
pub struct JsonRpcServer {
    session: LspSession,
}

impl JsonRpcServer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn handle_message(&mut self, message: &str) -> Vec<String> {
        let Ok(value) = serde_json::from_str::<Value>(message) else {
            return vec![error_response(Value::Null, -32700, "parse error")];
        };
        let id = value.get("id").cloned();
        let method = value.get("method").and_then(Value::as_str);
        let params = value.get("params").cloned().unwrap_or(Value::Null);
        let Some(method) = method else {
            return id
                .map(|id| error_response(id, -32600, "missing method"))
                .into_iter()
                .collect();
        };

        match (method, id) {
            ("initialize", Some(id)) => {
                self.initialize(&params);
                vec![success_response(id, initialize_result())]
            }
            ("initialized", _) => Vec::new(),
            ("shutdown", Some(id)) => vec![success_response(id, Value::Null)],
            ("exit", _) => Vec::new(),
            ("textDocument/didOpen", _) => {
                self.did_open(&params);
                self.publish_diagnostics_for_params(&params)
            }
            ("textDocument/didChange", _) => {
                self.did_change(&params);
                self.publish_diagnostics_for_params(&params)
            }
            ("textDocument/didClose", _) => {
                self.did_close(&params);
                Vec::new()
            }
            ("textDocument/hover", Some(id)) => {
                vec![success_response(
                    id,
                    self.hover(&params).unwrap_or(Value::Null),
                )]
            }
            ("textDocument/completion", Some(id)) => {
                vec![success_response(id, self.completion(&params))]
            }
            ("textDocument/signatureHelp", Some(id)) => {
                vec![success_response(
                    id,
                    self.signature_help(&params).unwrap_or(Value::Null),
                )]
            }
            ("textDocument/definition", Some(id)) => {
                vec![success_response(
                    id,
                    self.definition(&params).unwrap_or(Value::Null),
                )]
            }
            ("textDocument/references", Some(id)) => {
                vec![success_response(id, self.references(&params))]
            }
            ("textDocument/rename", Some(id)) => {
                vec![success_response(
                    id,
                    self.rename(&params).unwrap_or(Value::Null),
                )]
            }
            ("textDocument/codeAction", Some(id)) => {
                vec![success_response(id, self.code_actions(&params))]
            }
            ("textDocument/formatting", Some(id)) => {
                vec![success_response(id, self.formatting(&params))]
            }
            ("textDocument/semanticTokens/full", Some(id)) => {
                vec![success_response(id, self.semantic_tokens(&params))]
            }
            ("textDocument/inlayHint", Some(id)) => {
                vec![success_response(id, self.inlay_hints(&params))]
            }
            ("workspace/symbol", Some(id)) => {
                vec![success_response(id, self.workspace_symbols(&params))]
            }
            ("workspace/didChangeWorkspaceFolders", _) => {
                self.did_change_workspace_folders(&params);
                Vec::new()
            }
            (_, Some(id)) => vec![error_response(id, -32601, "method not found")],
            _ => Vec::new(),
        }
    }

    fn initialize(&mut self, params: &Value) {
        let roots = workspace_roots(params);
        for root in roots {
            let _ = self.session.open_project_dir(root);
        }
    }

    fn did_change_workspace_folders(&mut self, params: &Value) {
        let Some(added) = params
            .get("event")
            .and_then(|event| event.get("added"))
            .and_then(Value::as_array)
        else {
            return;
        };
        for folder in added {
            if let Some(root) = folder
                .get("uri")
                .and_then(Value::as_str)
                .and_then(uri_to_path)
            {
                let _ = self.session.open_project_dir(root);
            }
        }
    }

    fn did_open(&mut self, params: &Value) {
        let Some(document) = params.get("textDocument") else {
            return;
        };
        let Some(uri) = document.get("uri").and_then(Value::as_str) else {
            return;
        };
        let Some(text) = document.get("text").and_then(Value::as_str) else {
            return;
        };
        let path = uri_to_path(uri).unwrap_or_else(|| uri.to_owned());
        let _ = self.session.open_document(path, text);
    }

    fn did_change(&mut self, params: &Value) {
        let Some(uri) = text_document_uri(params) else {
            return;
        };
        let Some(text) = params
            .get("contentChanges")
            .and_then(Value::as_array)
            .and_then(|changes| changes.last())
            .and_then(|change| change.get("text"))
            .and_then(Value::as_str)
        else {
            return;
        };
        let path = uri_to_path(uri).unwrap_or_else(|| uri.to_owned());
        let _ = self.session.change_document(path, text);
    }

    fn did_close(&mut self, params: &Value) {
        let Some(uri) = text_document_uri(params) else {
            return;
        };
        let path = uri_to_path(uri).unwrap_or_else(|| uri.to_owned());
        let _ = self.session.close_document(path);
    }

    fn hover(&self, params: &Value) -> Option<Value> {
        let (file, position) = self.file_position(params)?;
        let card = self.session.hover_card_at(file, position)?;
        Some(hover_value(&card))
    }

    fn completion(&self, params: &Value) -> Value {
        let Some((file, position)) = self.file_position(params) else {
            return json!([]);
        };
        Value::Array(
            self.session
                .completions_at(file, position)
                .iter()
                .map(completion_value)
                .collect(),
        )
    }

    fn signature_help(&self, params: &Value) -> Option<Value> {
        let (file, position) = self.file_position(params)?;
        self.session
            .signature_help_at(file, position)
            .as_ref()
            .map(signature_value)
    }

    fn definition(&self, params: &Value) -> Option<Value> {
        let (file, position) = self.file_position(params)?;
        let definition = self.session.definition_at(file, position)?;
        let span = definition.span?;
        let range = crate::protocol::range(self.session.db(), span)?;
        Some(location_value(
            self.session.db().source(span.file)?.path.as_str(),
            range,
        ))
    }

    fn references(&self, params: &Value) -> Value {
        let Some((file, position)) = self.file_position(params) else {
            return json!([]);
        };
        Value::Array(
            self.session
                .references_at(file, position)
                .iter()
                .filter_map(|span| {
                    Some(location_value(
                        self.session.db().source(span.file)?.path.as_str(),
                        crate::protocol::range(self.session.db(), *span)?,
                    ))
                })
                .collect(),
        )
    }

    fn rename(&self, params: &Value) -> Option<Value> {
        let (file, position) = self.file_position(params)?;
        let new_name = params.get("newName").and_then(Value::as_str)?;
        self.session
            .rename_at(file, position, new_name)
            .as_ref()
            .and_then(|edit| workspace_edit_value(self.session.db(), edit))
    }

    fn code_actions(&self, params: &Value) -> Value {
        let Some(file) = self.file(params) else {
            return json!([]);
        };
        Value::Array(
            self.session
                .code_actions(file)
                .iter()
                .filter_map(|action| code_action_value(self.session.db(), action))
                .collect(),
        )
    }

    fn semantic_tokens(&self, params: &Value) -> Value {
        let Some(file) = self.file(params) else {
            return json!({ "data": [] });
        };
        json!({ "data": semantic_token_data(&self.session.semantic_tokens(file)) })
    }

    fn formatting(&self, params: &Value) -> Value {
        let Some(file) = self.file(params) else {
            return json!([]);
        };
        formatting_value(&self.session.formatting(file))
    }

    fn inlay_hints(&self, params: &Value) -> Value {
        let Some(file) = self.file(params) else {
            return json!([]);
        };
        Value::Array(
            self.session
                .inlay_hints(file)
                .iter()
                .map(inlay_hint_value)
                .collect(),
        )
    }

    fn workspace_symbols(&self, params: &Value) -> Value {
        let query = params.get("query").and_then(Value::as_str).unwrap_or("");
        Value::Array(
            self.session
                .workspace_symbols(query)
                .iter()
                .map(workspace_symbol_value)
                .collect(),
        )
    }

    fn publish_diagnostics_for_params(&self, params: &Value) -> Vec<String> {
        let Some(uri) = text_document_uri(params) else {
            return Vec::new();
        };
        let path = uri_to_path(uri).unwrap_or_else(|| uri.to_owned());
        let Some(file) = self.session.file_for_path(&path) else {
            return Vec::new();
        };
        vec![notification(
            "textDocument/publishDiagnostics",
            json!({
                "uri": path_to_uri(&path),
                "diagnostics": self.session.lsp_diagnostics(file).iter().map(diagnostic_value).collect::<Vec<_>>()
            }),
        )]
    }

    fn file_position(&self, params: &Value) -> Option<(tune_db::FileId, crate::Position)> {
        Some((self.file(params)?, position_param(params)?))
    }

    fn file(&self, params: &Value) -> Option<tune_db::FileId> {
        let uri = text_document_uri(params)?;
        let path = uri_to_path(uri).unwrap_or_else(|| uri.to_owned());
        self.session.file_for_path(path)
    }
}

fn workspace_roots(params: &Value) -> Vec<String> {
    let folders = params
        .get("workspaceFolders")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|folder| {
            folder
                .get("uri")
                .and_then(Value::as_str)
                .and_then(uri_to_path)
        })
        .collect::<Vec<_>>();
    if !folders.is_empty() {
        return folders;
    }
    params
        .get("rootUri")
        .and_then(Value::as_str)
        .and_then(uri_to_path)
        .or_else(|| {
            params
                .get("rootPath")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .into_iter()
        .collect()
}

pub fn run_stdio<R: BufRead, W: Write>(
    reader: &mut R,
    writer: &mut W,
    server: &mut JsonRpcServer,
) -> io::Result<()> {
    while let Some(message) = read_message(reader)? {
        for response in server.handle_message(&message) {
            write_message(writer, &response)?;
        }
        writer.flush()?;
    }
    Ok(())
}

fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length = None;
    let mut saw_header = false;
    loop {
        let mut line = String::new();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            return if saw_header {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete LSP header",
                ))
            } else {
                Ok(None)
            };
        }
        saw_header = true;
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }

    let Some(content_length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length",
        ));
    };
    let mut buffer = vec![0_u8; content_length];
    reader.read_exact(&mut buffer)?;
    String::from_utf8(buffer)
        .map(Some)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub fn write_message(writer: &mut impl Write, message: &str) -> io::Result<()> {
    write!(
        writer,
        "Content-Length: {}\r\n\r\n{}",
        message.len(),
        message
    )
}
