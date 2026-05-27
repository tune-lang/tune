use tune_db::{FileId, TuneDb};
use tune_diagnostics::Diagnostic;

pub fn handle() {
    // LSP diagnostics handler skeleton. This should query compiler facts, not infer.
}

#[must_use]
pub fn diagnostics_for_file(db: &TuneDb, file: FileId) -> Vec<Diagnostic> {
    db.analyze_file(file)
        .map_or_else(Vec::new, |analysis| analysis.diagnostics())
}
