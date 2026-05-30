pub fn handle() {
    // LSP inlay handler skeleton. This should query compiler facts, not infer.
}

use tune_db::{FileId, TuneDb};
use tune_hir::item::ItemKind;
use tune_resolve::{CompilerFactPayload, FactOwner, LocalKind};
use tune_shape::{BindingKey, Shape};

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
    let mut hints = Vec::new();
    for (index, item) in analysis.module.items.iter().enumerate() {
        if item.kind == ItemKind::Let
            && item.shape.is_none()
            && let Some(shape) = item
                .body
                .as_ref()
                .and_then(|body| expr_shape(&analysis, body.id))
                .or_else(|| {
                    analysis
                        .shape
                        .get(index)
                        .map(|shape| &shape.item_current_shape)
                })
            && !is_suppressed_shape(shape)
            && let Some(span) = name_span_for_owner(&analysis, FactOwner::Item(item.id))
        {
            push_type_hint(db, &mut hints, span, shape);
        }
    }
    for local in &analysis.resolved.locals {
        if local.kind != LocalKind::Let {
            continue;
        }
        let Some(span) = local.span else {
            continue;
        };
        let Some(shape) = local
            .expr
            .and_then(|expr| expr_shape(&analysis, expr))
            .or_else(|| binding_shape(&analysis, BindingKey::Local(local.id)))
        else {
            continue;
        };
        if !is_suppressed_shape(shape) {
            push_type_hint(db, &mut hints, span, shape);
        }
    }
    hints.sort_by_key(|hint| {
        (
            hint.position.line,
            hint.position.character,
            hint.label.clone(),
        )
    });
    hints
}

fn expr_shape(analysis: &tune_db::ModuleAnalysis, expr: tune_hir::ExprId) -> Option<&Shape> {
    analysis
        .shape
        .iter()
        .flat_map(|shape| shape.expr_shapes.iter())
        .find(|shape| shape.expr == expr)
        .map(|shape| &shape.shape)
}

fn binding_shape(analysis: &tune_db::ModuleAnalysis, key: BindingKey) -> Option<&Shape> {
    analysis
        .shape
        .iter()
        .find_map(|shape| shape.frame.get(key))
        .map(|binding| &binding.current_shape)
}

fn name_span_for_owner(
    analysis: &tune_db::ModuleAnalysis,
    owner: FactOwner,
) -> Option<tune_diagnostics::Span> {
    analysis
        .resolved
        .facts
        .iter()
        .find(|fact| fact.owner == owner && matches!(fact.payload, CompilerFactPayload::Name(_)))
        .and_then(|fact| fact.span)
}

fn push_type_hint(
    db: &TuneDb,
    hints: &mut Vec<InlayHint>,
    span: tune_diagnostics::Span,
    shape: &Shape,
) {
    let Some(position) = crate::protocol::range(db, span).map(|range| range.end) else {
        return;
    };
    hints.push(InlayHint {
        position,
        label: format!(": {}", crate::hover::semantic_shape_text(shape)),
        kind: InlayHintKind::Type,
    });
}

fn is_suppressed_shape(shape: &Shape) -> bool {
    matches!(shape, Shape::Hole | Shape::Never | Shape::Unit)
}
