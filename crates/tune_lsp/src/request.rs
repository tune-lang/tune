use tune_db::FileId;

use crate::code_action::CodeAction;
use crate::completion::CompletionItem;
use crate::hover::HoverCard;
use crate::inlay::InlayHint;
use crate::protocol::WorkspaceEdit;
use crate::semantic_tokens::SemanticToken;
use crate::signature::SignatureHelp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspRequest {
    Hover {
        file: FileId,
        position: crate::Position,
    },
    Completion {
        file: FileId,
        position: crate::Position,
    },
    SignatureHelp {
        file: FileId,
        position: crate::Position,
    },
    Definition {
        file: FileId,
        position: crate::Position,
    },
    References {
        file: FileId,
        position: crate::Position,
    },
    Rename {
        file: FileId,
        position: crate::Position,
        new_name: String,
    },
    InlayHints {
        file: FileId,
    },
    SemanticTokens {
        file: FileId,
    },
    CodeActions {
        file: FileId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspResponse {
    Hover(Option<HoverCard>),
    Completion(Vec<CompletionItem>),
    SignatureHelp(Option<SignatureHelp>),
    Definition(Option<tune_db::SemanticDefinition>),
    References(Vec<tune_diagnostics::Span>),
    Rename(Option<WorkspaceEdit>),
    InlayHints(Vec<InlayHint>),
    SemanticTokens(Vec<SemanticToken>),
    CodeActions(Vec<CodeAction>),
}
