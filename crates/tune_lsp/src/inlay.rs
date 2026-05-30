pub fn handle() {
    // LSP inlay handler skeleton. This should query compiler facts, not infer.
}

use tune_db::{FileId, TuneDb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    Type,
    Parameter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayHint {
    pub position: crate::Position,
    pub label: String,
    pub kind: InlayHintKind,
}

#[must_use]
pub fn hints_for_file(db: &TuneDb, file: FileId) -> Vec<InlayHint> {
    let Some(analysis) = db.analyze_file(file) else {
        return Vec::new();
    };
    let expr_spans = db.semantic_exprs(file).unwrap_or_default();
    analysis
        .shape
        .iter()
        .flat_map(|shape| shape.expr_shapes.iter())
        .filter_map(|expr| {
            let span = expr_spans
                .iter()
                .find(|candidate| candidate.id == expr.expr)
                .and_then(|candidate| candidate.span)?;
            Some(InlayHint {
                position: crate::protocol::range(db, span)?.end,
                label: format!(": {}", crate::hover::semantic_shape_text(&expr.shape)),
                kind: InlayHintKind::Type,
            })
        })
        .collect()
}
