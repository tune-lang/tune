use serde_json::{Value, json};

use crate::code_action::CodeAction;
use crate::completion::{CompletionItem, CompletionKind};
use crate::hover::HoverCard;
use crate::inlay::{InlayHint, InlayHintKind};
use crate::protocol::{Range, TextEdit, WorkspaceEdit};
use crate::semantic_tokens::{SemanticToken, SemanticTokenKind};
use crate::signature::SignatureHelp;
use crate::workspace::{WorkspaceSymbol, WorkspaceSymbolKind};

pub(super) fn success_response(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

pub(super) fn error_response(id: Value, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
    .to_string()
}

pub(super) fn notification(method: &str, params: Value) -> String {
    json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string()
}

pub(super) fn initialize_result() -> Value {
    json!({
        "capabilities": {
            "textDocumentSync": 1,
            "hoverProvider": true,
            "completionProvider": { "triggerCharacters": ["."] },
            "signatureHelpProvider": { "triggerCharacters": ["(", ","] },
            "definitionProvider": true,
            "referencesProvider": true,
            "renameProvider": true,
            "codeActionProvider": true,
            "workspaceSymbolProvider": true,
            "inlayHintProvider": true,
            "semanticTokensProvider": {
                "legend": {
                    "tokenTypes": [
                        "function",
                        "type",
                        "variable",
                        "parameter",
                        "property",
                        "enumMember",
                        "namespace"
                    ],
                    "tokenModifiers": []
                },
                "full": true
            }
        }
    })
}

pub(super) fn text_document_uri(params: &Value) -> Option<&str> {
    params
        .get("textDocument")
        .and_then(|document| document.get("uri"))
        .and_then(Value::as_str)
}

pub(super) fn position_param(params: &Value) -> Option<crate::Position> {
    let position = params.get("position")?;
    Some(crate::Position {
        line: u32::try_from(position.get("line")?.as_u64()?).ok()?,
        character: u32::try_from(position.get("character")?.as_u64()?).ok()?,
    })
}

pub(super) fn hover_value(card: &HoverCard) -> Value {
    json!({ "contents": { "kind": "markdown", "value": card.markdown() } })
}

pub(super) fn completion_value(item: &CompletionItem) -> Value {
    json!({
        "label": item.label,
        "kind": completion_kind(item.kind),
        "detail": item.detail,
        "documentation": item.documentation.as_ref().map(|doc| {
            json!({ "kind": "markdown", "value": doc })
        })
    })
}

fn completion_kind(kind: CompletionKind) -> u32 {
    match kind {
        CompletionKind::Function => 3,
        CompletionKind::Type => 7,
        CompletionKind::Value => 6,
        CompletionKind::Module => 9,
        CompletionKind::Keyword => 14,
    }
}

pub(super) fn signature_value(help: &SignatureHelp) -> Value {
    json!({
        "signatures": [{ "label": help.signature }],
        "activeSignature": 0,
        "activeParameter": help.active_parameter.unwrap_or(0)
    })
}

pub(super) fn location_value(path: &str, range: Range) -> Value {
    json!({ "uri": path_to_uri(path), "range": range_value(range) })
}

pub(super) fn code_action_value(db: &tune_db::TuneDb, action: &CodeAction) -> Option<Value> {
    Some(json!({
        "title": action.title,
        "kind": "quickfix",
        "edit": action.edit.as_ref().and_then(|edit| workspace_edit_value(db, edit))
    }))
}

pub(super) fn workspace_edit_value(db: &tune_db::TuneDb, edit: &WorkspaceEdit) -> Option<Value> {
    let source = db.source(edit.file)?;
    Some(json!({
        "changes": {
            path_to_uri(&source.path): edit.edits.iter().map(text_edit_value).collect::<Vec<_>>()
        }
    }))
}

fn text_edit_value(edit: &TextEdit) -> Value {
    json!({ "range": range_value(edit.range), "newText": edit.replacement })
}

pub(super) fn diagnostic_value(diagnostic: &crate::LspDiagnostic) -> Value {
    json!({
        "range": range_value(diagnostic.range),
        "severity": match diagnostic.severity {
            crate::DiagnosticSeverity::Error => 1,
            crate::DiagnosticSeverity::Warning => 2,
            crate::DiagnosticSeverity::Information => 3,
        },
        "code": diagnostic.code,
        "message": diagnostic.message
    })
}

pub(super) fn inlay_hint_value(hint: &InlayHint) -> Value {
    json!({
        "position": position_value(hint.position),
        "label": hint.label,
        "kind": match hint.kind {
            InlayHintKind::Type => 1,
            InlayHintKind::Parameter => 2,
        }
    })
}

pub(super) fn workspace_symbol_value(symbol: &WorkspaceSymbol) -> Value {
    json!({
        "name": symbol.name,
        "kind": workspace_symbol_kind(symbol.kind),
        "location": {
            "uri": path_to_uri(&symbol.path),
            "range": range_value(Range {
                start: crate::Position { line: 0, character: 0 },
                end: crate::Position { line: 0, character: 0 },
            })
        }
    })
}

fn workspace_symbol_kind(kind: WorkspaceSymbolKind) -> u32 {
    match kind {
        WorkspaceSymbolKind::Function => 12,
        WorkspaceSymbolKind::Type => 5,
        WorkspaceSymbolKind::Value => 13,
        WorkspaceSymbolKind::Module => 2,
    }
}

pub(super) fn semantic_token_data(tokens: &[SemanticToken]) -> Vec<u32> {
    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut previous_line = 0_u32;
    let mut previous_start = 0_u32;
    for token in tokens {
        let line = token.range.start.line;
        let start = token.range.start.character;
        let delta_line = line.saturating_sub(previous_line);
        let delta_start = if delta_line == 0 {
            start.saturating_sub(previous_start)
        } else {
            start
        };
        let length = token
            .range
            .end
            .character
            .saturating_sub(token.range.start.character);
        data.extend([
            delta_line,
            delta_start,
            length,
            semantic_token_kind(token.kind),
            0,
        ]);
        previous_line = line;
        previous_start = start;
    }
    data
}

fn semantic_token_kind(kind: SemanticTokenKind) -> u32 {
    match kind {
        SemanticTokenKind::Function => 0,
        SemanticTokenKind::Type => 1,
        SemanticTokenKind::Variable => 2,
        SemanticTokenKind::Parameter => 3,
        SemanticTokenKind::Property => 4,
        SemanticTokenKind::EnumMember => 5,
        SemanticTokenKind::Module => 6,
    }
}

fn range_value(range: Range) -> Value {
    json!({ "start": position_value(range.start), "end": position_value(range.end) })
}

fn position_value(position: crate::Position) -> Value {
    json!({ "line": position.line, "character": position.character })
}

pub(super) fn uri_to_path(uri: &str) -> Option<String> {
    let path = uri.strip_prefix("file://")?;
    Some(percent_decode(path))
}

pub(super) fn path_to_uri(path: &str) -> String {
    if path.starts_with("file://") {
        return path.to_owned();
    }
    format!("file://{}", path.replace(' ', "%20"))
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Some(byte) = hex_byte(bytes[index + 1], bytes[index + 2])
        {
            out.push(char::from(byte));
            index += 3;
            continue;
        }
        out.push(char::from(bytes[index]));
        index += 1;
    }
    out
}

fn hex_byte(high: u8, low: u8) -> Option<u8> {
    hex_digit(high)?
        .checked_mul(16)?
        .checked_add(hex_digit(low)?)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
