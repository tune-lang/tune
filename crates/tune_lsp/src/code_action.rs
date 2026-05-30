use tune_db::{FileId, TuneDb};
use tune_diagnostics::{Diagnostic, codes};
use tune_hir::item::{Item, ItemKind, Visibility};
use tune_shape::Shape;

use crate::protocol::{TextEdit, WorkspaceEdit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAction {
    pub title: String,
    pub edit: Option<WorkspaceEdit>,
}

#[must_use]
pub fn actions_for_file(db: &TuneDb, file: FileId) -> Vec<CodeAction> {
    let Some(analysis) = db.analyze_file(file) else {
        return Vec::new();
    };
    analysis
        .diagnostics()
        .iter()
        .flat_map(|diagnostic| action_for_diagnostic(db, file, &analysis, diagnostic))
        .collect()
}

fn action_for_diagnostic(
    db: &TuneDb,
    file: FileId,
    analysis: &tune_db::ModuleAnalysis,
    diagnostic: &Diagnostic,
) -> Vec<CodeAction> {
    if diagnostic.code == codes::PUBLIC_API_INFERENCE {
        return materialize_public_signature(db, file, analysis, diagnostic)
            .into_iter()
            .collect();
    }
    Vec::new()
}

fn materialize_public_signature(
    db: &TuneDb,
    file: FileId,
    analysis: &tune_db::ModuleAnalysis,
    diagnostic: &Diagnostic,
) -> Option<CodeAction> {
    let item = analysis
        .module
        .items
        .iter()
        .find(|item| item.span == Some(diagnostic.primary_span()))?;
    if item.visibility != Visibility::Public {
        return None;
    }
    let replacement = public_head(analysis, item)?;
    let edit_span = head_span(db, item)?;
    Some(CodeAction {
        title: "Insert inferred public signature".to_owned(),
        edit: Some(WorkspaceEdit {
            file,
            edits: vec![TextEdit {
                range: crate::protocol::range(db, edit_span)?,
                replacement,
            }],
        }),
    })
}

fn public_head(analysis: &tune_db::ModuleAnalysis, item: &Item) -> Option<String> {
    match item.kind {
        ItemKind::CallableDecl => {
            let index = analysis
                .module
                .items
                .iter()
                .position(|candidate| candidate.id == item.id)?;
            let signature = analysis.shape.get(index)?.inferred_signature.as_ref()?;
            let name = item.name.as_deref()?;
            let params = item
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    let name = param.name.clone().unwrap_or_else(|| format!("arg{index}"));
                    let shape = signature
                        .params
                        .get(index)
                        .map(crate::hover::semantic_shape_text)
                        .unwrap_or_else(|| "_".to_owned());
                    format!("{name}: {shape}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!(
                "pub let {name}({params}): {} ",
                crate::hover::semantic_shape_text(&signature.ret)
            ))
        }
        ItemKind::Let => {
            let name = item.name.as_deref()?;
            let shape = item_current_shape(analysis, item)?;
            Some(format!(
                "pub let {name}: {} ",
                crate::hover::semantic_shape_text(&shape)
            ))
        }
        _ => None,
    }
}

fn item_current_shape(analysis: &tune_db::ModuleAnalysis, item: &Item) -> Option<Shape> {
    analysis
        .module
        .items
        .iter()
        .position(|candidate| candidate.id == item.id)
        .and_then(|index| analysis.shape.get(index))
        .map(|shape| shape.item_current_shape.clone())
}

fn head_span(db: &TuneDb, item: &Item) -> Option<tune_diagnostics::Span> {
    let span = item.span?;
    let source = db.source(span.file)?;
    let start = usize::try_from(span.start.get()).ok()?;
    let end = usize::try_from(span.end.get()).ok()?;
    let text = source.text.get(start..end)?;
    let equals = text.find('=')?;
    let edit_end = u32::try_from(start + equals).ok()?;
    tune_diagnostics::Span::checked(
        span.file,
        span.start,
        tune_diagnostics::ByteOffset::new(edit_end),
    )
}
