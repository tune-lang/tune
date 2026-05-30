use tune_db::{FileId, TuneDb};
use tune_diagnostics::Diagnostic;
use tune_diagnostics::Span;
use tune_resolve::{CompilerFact, FactOwner};

use crate::{
    completion::{self, CompletionItem},
    diagnostics,
    hover::{self, HoverCard},
    protocol::LspDiagnostic,
    signature::{self, SignatureHelp},
};

pub fn handle() {
    // LSP server handler skeleton. This should query compiler facts, not infer.
}

#[derive(Default)]
pub struct LspSession {
    db: TuneDb,
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
        self.db.add_file(path, text)
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
        let Some(offset) = crate::protocol::byte_offset(&self.db, file, position) else {
            return Vec::new();
        };
        let Some(cursor) = self.db.semantic_at(file, offset) else {
            return Vec::new();
        };
        let Some(reference) = cursor.reference else {
            return Vec::new();
        };
        let Some(analysis) = self.db.analyze_file(file) else {
            return Vec::new();
        };
        analysis
            .resolved
            .name_refs
            .iter()
            .filter(|candidate| candidate.target == reference.target)
            .filter_map(|candidate| candidate.span)
            .chain(reference.definition.and_then(|definition| definition.span))
            .collect()
    }

    #[must_use]
    pub const fn db(&self) -> &TuneDb {
        &self.db
    }
}
