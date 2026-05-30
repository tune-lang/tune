use tune_db::{FileId, TuneDb};
use tune_diagnostics::Diagnostic;

use crate::protocol::LspDiagnostic;

pub fn handle() {
    // LSP diagnostics handler skeleton. This should query compiler facts, not infer.
}

#[must_use]
pub fn diagnostics_for_file(db: &TuneDb, file: FileId) -> Vec<Diagnostic> {
    db.analyze_file(file)
        .map_or_else(Vec::new, |analysis| analysis.diagnostics())
}

#[must_use]
pub fn lsp_diagnostics_for_file(db: &TuneDb, file: FileId) -> Vec<LspDiagnostic> {
    diagnostics_for_file(db, file)
        .iter()
        .filter_map(|diagnostic| crate::protocol::diagnostic(db, diagnostic))
        .collect()
}
