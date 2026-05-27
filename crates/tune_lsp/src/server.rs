use tune_db::{FileId, TuneDb};
use tune_diagnostics::Diagnostic;
use tune_resolve::{CompilerFact, FactOwner};

use crate::{diagnostics, hover};

pub fn handle() {
    // LSP server handler skeleton. This should query compiler facts, not infer.
}

#[derive(Default)]
pub struct LspSession {
    db: TuneDb,
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
    pub fn facts_for_owner(&self, file: FileId, owner: FactOwner) -> Vec<CompilerFact> {
        hover::facts_for_owner(&self.db, file, owner)
    }

    #[must_use]
    pub const fn db(&self) -> &TuneDb {
        &self.db
    }
}
