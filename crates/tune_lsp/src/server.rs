use tune_db::{FileId, TuneDb};
use tune_diagnostics::Diagnostic;
use tune_diagnostics::Span;
use tune_resolve::{CompilerFact, FactOwner};

use crate::{
    code_action::{self, CodeAction},
    completion::{self, CompletionItem},
    diagnostics,
    document::DocumentSet,
    hover::{self, HoverCard},
    inlay::{self, InlayHint},
    protocol::LspDiagnostic,
    rename,
    request::{LspRequest, LspResponse},
    semantic_tokens::{self, SemanticToken},
    signature::{self, SignatureHelp},
};

pub fn handle() {
    // LSP server handler skeleton. This should query compiler facts, not infer.
}

#[derive(Default)]
pub struct LspSession {
    db: TuneDb,
    documents: DocumentSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticHover {
    pub diagnostic: LspDiagnostic,
    pub markdown: String,
}

impl LspSession {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: impl Into<String>, text: impl Into<String>) -> Option<FileId> {
        self.open_document(path, text)
    }

    pub fn open_document(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Option<FileId> {
        self.documents.open(&mut self.db, path, text)
    }

    pub fn change_document(
        &mut self,
        path: impl AsRef<str>,
        text: impl Into<String>,
    ) -> Option<FileId> {
        self.documents.change(&mut self.db, path, text)
    }

    pub fn close_document(&mut self, path: impl AsRef<str>) -> Option<FileId> {
        self.documents.close(path)
    }

    #[must_use]
    pub fn file_for_path(&self, path: impl AsRef<str>) -> Option<FileId> {
        self.documents
            .file(path.as_ref())
            .or_else(|| self.db.file_by_path(path.as_ref()))
    }

    #[must_use]
    pub fn diagnostics(&self, file: FileId) -> Vec<Diagnostic> {
        diagnostics::diagnostics_for_file(&self.db, file)
    }

    #[must_use]
    pub fn lsp_diagnostics(&self, file: FileId) -> Vec<LspDiagnostic> {
        diagnostics::lsp_diagnostics_for_file(&self.db, file)
    }

    #[must_use]
    pub fn diagnostic_hovers(&self, file: FileId) -> Vec<DiagnosticHover> {
        diagnostics::diagnostics_for_file(&self.db, file)
            .iter()
            .filter_map(|diagnostic| {
                Some(DiagnosticHover {
                    diagnostic: crate::protocol::diagnostic(&self.db, diagnostic)?,
                    markdown: crate::protocol::diagnostic_hover(diagnostic),
                })
            })
            .collect()
    }

    #[must_use]
    pub fn completions(&self, file: FileId) -> Vec<CompletionItem> {
        completion::items_for_file(&self.db, file)
    }

    #[must_use]
    pub fn completions_at(&self, file: FileId, position: crate::Position) -> Vec<CompletionItem> {
        completion::items_at(&self.db, file, position)
    }

    #[must_use]
    pub fn facts_for_owner(&self, file: FileId, owner: FactOwner) -> Vec<CompilerFact> {
        hover::facts_for_owner(&self.db, file, owner)
    }

    #[must_use]
    pub fn hover_card(&self, file: FileId, owner: FactOwner) -> Option<HoverCard> {
        hover::hover_card(&self.db, file, owner)
    }

    #[must_use]
    pub fn hover_card_at(&self, file: FileId, position: crate::Position) -> Option<HoverCard> {
        hover::hover_card_at(&self.db, file, position)
    }

    #[must_use]
    pub fn signature_help_at(
        &self,
        file: FileId,
        position: crate::Position,
    ) -> Option<SignatureHelp> {
        signature::signature_help_at(&self.db, file, position)
    }

    #[must_use]
    pub fn definition_at(
        &self,
        file: FileId,
        position: crate::Position,
    ) -> Option<tune_db::SemanticDefinition> {
        let offset = crate::protocol::byte_offset(&self.db, file, position)?;
        self.db.semantic_at(file, offset)?.reference?.definition
    }

    #[must_use]
    pub fn references_at(&self, file: FileId, position: crate::Position) -> Vec<Span> {
        rename::reference_spans_at(&self.db, file, position)
    }

    #[must_use]
    pub fn rename_at(
        &self,
        file: FileId,
        position: crate::Position,
        new_name: &str,
    ) -> Option<crate::WorkspaceEdit> {
        rename::rename_at(&self.db, file, position, new_name)
    }

    #[must_use]
    pub fn inlay_hints(&self, file: FileId) -> Vec<InlayHint> {
        inlay::hints_for_file(&self.db, file)
    }

    #[must_use]
    pub fn semantic_tokens(&self, file: FileId) -> Vec<SemanticToken> {
        semantic_tokens::tokens_for_file(&self.db, file)
    }

    #[must_use]
    pub fn code_actions(&self, file: FileId) -> Vec<CodeAction> {
        code_action::actions_for_file(&self.db, file)
    }

    #[must_use]
    pub fn handle_request(&self, request: LspRequest) -> LspResponse {
        match request {
            LspRequest::Hover { file, position } => {
                LspResponse::Hover(self.hover_card_at(file, position))
            }
            LspRequest::Completion { file, position } => {
                LspResponse::Completion(self.completions_at(file, position))
            }
            LspRequest::SignatureHelp { file, position } => {
                LspResponse::SignatureHelp(self.signature_help_at(file, position))
            }
            LspRequest::Definition { file, position } => {
                LspResponse::Definition(self.definition_at(file, position))
            }
            LspRequest::References { file, position } => {
                LspResponse::References(self.references_at(file, position))
            }
            LspRequest::Rename {
                file,
                position,
                new_name,
            } => LspResponse::Rename(self.rename_at(file, position, &new_name)),
            LspRequest::InlayHints { file } => LspResponse::InlayHints(self.inlay_hints(file)),
            LspRequest::SemanticTokens { file } => {
                LspResponse::SemanticTokens(self.semantic_tokens(file))
            }
            LspRequest::CodeActions { file } => LspResponse::CodeActions(self.code_actions(file)),
        }
    }

    #[must_use]
    pub const fn db(&self) -> &TuneDb {
        &self.db
    }
}
