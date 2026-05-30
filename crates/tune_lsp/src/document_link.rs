use tune_db::{FileId, TuneDb};
use tune_diagnostics::{ByteOffset, Span};
use tune_hir::item::ItemKind;

use crate::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentLink {
    pub range: Range,
    pub target: String,
}

pub fn links_for_file(db: &TuneDb, file: FileId) -> Vec<DocumentLink> {
    let Some(analysis) = db.analyze_file(file) else {
        return Vec::new();
    };
    analysis
        .module
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::Import)
        .filter_map(|item| {
            let import = item.import.as_ref()?;
            let target = db.file_by_path(&import.path)?;
            let target_path = db.source(target)?.path.clone();
            Some(DocumentLink {
                range: import_path_range(db, item.span?, &import.path)?,
                target: target_path,
            })
        })
        .collect()
}

fn import_path_range(db: &TuneDb, span: Span, path: &str) -> Option<Range> {
    let source = db.source(span.file)?;
    let start = usize::try_from(span.start.get()).ok()?;
    let end = usize::try_from(span.end.get()).ok()?;
    let item_text = source.text.get(start..end)?;
    let path_start = item_text.find(path)?;
    let range_start = start.checked_add(path_start)?;
    let range_end = range_start.checked_add(path.len())?;
    let span = Span::checked(
        span.file,
        ByteOffset::new(u32::try_from(range_start).ok()?),
        ByteOffset::new(u32::try_from(range_end).ok()?),
    )?;
    crate::protocol::range(db, span)
}
