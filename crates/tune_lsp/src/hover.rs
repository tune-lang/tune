use tune_db::{FileId, TuneDb};
use tune_resolve::{CompilerFact, FactOwner};

pub fn handle() {
    // LSP hover handler skeleton. This should query compiler facts, not infer.
}

#[must_use]
pub fn facts_for_owner(db: &TuneDb, file: FileId, owner: FactOwner) -> Vec<CompilerFact> {
    db.analyze_file(file).map_or_else(Vec::new, |analysis| {
        analysis
            .resolved
            .facts
            .into_iter()
            .filter(|fact| fact.owner == owner)
            .collect()
    })
}
