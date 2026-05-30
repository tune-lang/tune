use dyno_project::{ProjectSourceLoadError, ProjectSources};
use std::path::{Path, PathBuf};
use tune_db::{FileId, TuneDb};
use tune_diagnostics::Diagnostic;
use tune_diagnostics::{ByteOffset, Span};
use tune_resolve::{CompilerFact, FactOwner};

use crate::{
    code_action::{self, CodeAction},
    completion::{self, CompletionItem},
    diagnostics,
    document::DocumentSet,
    hover::{self, HoverCard},
    inlay::{self, InlayHint},
    protocol::{LspDiagnostic, TextEdit},
    rename,
    request::{LspRequest, LspResponse},
    semantic_tokens::{self, SemanticToken},
    signature::{self, SignatureHelp},
    workspace::{WorkspaceIndex, WorkspaceSymbol},
};

pub fn handle() {
    // LSP server handler skeleton. This should query compiler facts, not infer.
}

#[derive(Default)]
pub struct LspSession {
    db: TuneDb,
    documents: DocumentSet,
    workspace: WorkspaceIndex,
    project_root: Option<PathBuf>,
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
        let path = self.normalize_path(path.into());
        let file = self.documents.open(&mut self.db, path, text)?;
        self.rebuild_workspace_index();
        Some(file)
    }

    pub fn open_project_dir(
        &mut self,
        root: impl AsRef<Path>,
    ) -> Result<Vec<FileId>, ProjectSourceLoadError> {
        let sources = dyno_project::load_project_dir(root)?;
        Ok(self.open_project_sources(sources))
    }

    pub fn open_project_manifest(
        &mut self,
        manifest_path: impl AsRef<Path>,
    ) -> Result<Vec<FileId>, ProjectSourceLoadError> {
        let sources = dyno_project::load_project_manifest(manifest_path)?;
        Ok(self.open_project_sources(sources))
    }

    pub fn open_project_sources(&mut self, sources: ProjectSources) -> Vec<FileId> {
        self.project_root = Some(sources.root);
        sources
            .sources
            .into_iter()
            .filter_map(|(path, text)| self.open_document(path, text))
            .collect()
    }

    pub fn change_document(
        &mut self,
        path: impl AsRef<str>,
        text: impl Into<String>,
    ) -> Option<FileId> {
        let path = self.normalize_path(path.as_ref());
        let file = self.documents.change(&mut self.db, path, text)?;
        self.rebuild_workspace_index();
        Some(file)
    }

    pub fn close_document(&mut self, path: impl AsRef<str>) -> Option<FileId> {
        let path = self.normalize_path(path.as_ref());
        self.documents.close(path)
    }

    #[must_use]
    pub fn file_for_path(&self, path: impl AsRef<str>) -> Option<FileId> {
        let path = self.normalize_path(path.as_ref());
        self.documents
            .file(&path)
            .or_else(|| self.db.file_by_path(&path))
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
        code_action::actions_for_file_with_index(&self.db, file, Some(&self.workspace))
    }

    #[must_use]
    pub fn formatting(&self, file: FileId) -> Vec<TextEdit> {
        let Some(source) = self.db.source(file) else {
            return Vec::new();
        };
        let formatted = tune_fmt::format_source(&source.text);
        if formatted == source.text {
            return Vec::new();
        }
        let Ok(end) = u32::try_from(source.text.len()) else {
            return Vec::new();
        };
        let Some(range) =
            tune_diagnostics::Span::checked(file, ByteOffset::new(0), ByteOffset::new(end))
                .and_then(|span| crate::protocol::range(&self.db, span))
        else {
            return Vec::new();
        };
        vec![TextEdit {
            range,
            replacement: formatted,
        }]
    }

    #[must_use]
    pub fn workspace_symbols(&self, query: &str) -> Vec<WorkspaceSymbol> {
        self.workspace
            .symbols()
            .iter()
            .filter(|symbol| query.is_empty() || symbol.name.contains(query))
            .cloned()
            .collect()
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
            LspRequest::Formatting { file } => LspResponse::Formatting(self.formatting(file)),
            LspRequest::WorkspaceSymbols { query } => {
                LspResponse::WorkspaceSymbols(self.workspace_symbols(&query))
            }
        }
    }

    #[must_use]
    pub const fn db(&self) -> &TuneDb {
        &self.db
    }

    fn rebuild_workspace_index(&mut self) {
        self.workspace.rebuild(&self.db);
    }

    fn normalize_path(&self, path: impl AsRef<str>) -> String {
        let path = path.as_ref();
        let Some(root) = &self.project_root else {
            return path.to_owned();
        };
        let Ok(relative) = Path::new(path).strip_prefix(root) else {
            return path.to_owned();
        };
        relative
            .to_string_lossy()
            .trim_start_matches("./")
            .to_owned()
    }
}
