use tune_db::{FileId, TuneDb};
use tune_diagnostics::Span;

use crate::protocol::{TextEdit, WorkspaceEdit};

#[must_use]
pub fn rename_at(
    db: &TuneDb,
    file: FileId,
    position: crate::Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    if !is_valid_user_name(new_name) {
        return None;
    }
    let spans = reference_spans_at(db, file, position);
    if spans.is_empty() {
        return None;
    }
    let edits = spans
        .into_iter()
        .filter_map(|span| {
            Some(TextEdit {
                range: crate::protocol::range(db, span)?,
                replacement: new_name.to_owned(),
            })
        })
        .collect::<Vec<_>>();
    (!edits.is_empty()).then_some(WorkspaceEdit { file, edits })
}

#[must_use]
pub fn reference_spans_at(db: &TuneDb, file: FileId, position: crate::Position) -> Vec<Span> {
    let Some(offset) = crate::protocol::byte_offset(db, file, position) else {
        return Vec::new();
    };
    let Some(cursor) = db.semantic_at(file, offset) else {
        return Vec::new();
    };
    let Some(reference) = cursor.reference else {
        return Vec::new();
    };
    let Some(analysis) = db.analyze_file(file) else {
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

fn is_valid_user_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && !name.starts_with("__")
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}
