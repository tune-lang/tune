use tune_db::{FileId, TuneDb};
use tune_hir::item::{Item, ItemKind, StructMember};

use crate::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentSymbolKind {
    Function,
    Struct,
    Enum,
    Field,
    Property,
    Module,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    pub name: String,
    pub kind: DocumentSymbolKind,
    pub range: Range,
    pub selection_range: Range,
    pub detail: Option<String>,
    pub children: Vec<DocumentSymbol>,
}

pub fn symbols_for_file(db: &TuneDb, file: FileId) -> Vec<DocumentSymbol> {
    let Some(analysis) = db.analyze_file(file) else {
        return Vec::new();
    };
    analysis
        .module
        .items
        .iter()
        .filter_map(|item| item_symbol(db, item))
        .collect()
}

fn item_symbol(db: &TuneDb, item: &Item) -> Option<DocumentSymbol> {
    let range = item
        .span
        .and_then(|span| crate::protocol::range(db, span))?;
    let name = item
        .name
        .clone()
        .or_else(|| item.import.as_ref().map(|import| import.path.clone()))?;
    Some(DocumentSymbol {
        name,
        kind: item_kind(item.kind),
        range,
        selection_range: range,
        detail: crate::hover::hover_card(
            db,
            item.span?.file,
            tune_resolve::FactOwner::Item(item.id),
        )
        .and_then(|hover| hover.signature),
        children: item
            .struct_members
            .iter()
            .filter_map(|member| member_symbol(db, member))
            .chain(item.variants.iter().filter_map(|variant| {
                let range = variant
                    .span
                    .and_then(|span| crate::protocol::range(db, span))?;
                Some(DocumentSymbol {
                    name: variant.name.clone()?,
                    kind: DocumentSymbolKind::Property,
                    range,
                    selection_range: range,
                    detail: None,
                    children: Vec::new(),
                })
            }))
            .collect(),
    })
}

fn member_symbol(db: &TuneDb, member: &StructMember) -> Option<DocumentSymbol> {
    match member {
        StructMember::Field(field) => {
            let range = field
                .span
                .and_then(|span| crate::protocol::range(db, span))?;
            Some(DocumentSymbol {
                name: field.name.clone()?,
                kind: DocumentSymbolKind::Field,
                range,
                selection_range: range,
                detail: None,
                children: Vec::new(),
            })
        }
        StructMember::Callable(callable) => {
            let range = callable
                .span
                .and_then(|span| crate::protocol::range(db, span))?;
            Some(DocumentSymbol {
                name: callable.name.clone()?,
                kind: DocumentSymbolKind::Function,
                range,
                selection_range: range,
                detail: None,
                children: Vec::new(),
            })
        }
        StructMember::SequenceMaterializer(materializer) => {
            let range = materializer
                .span
                .and_then(|span| crate::protocol::range(db, span))?;
            Some(DocumentSymbol {
                name: "[]".to_owned(),
                kind: DocumentSymbolKind::Function,
                range,
                selection_range: range,
                detail: None,
                children: Vec::new(),
            })
        }
        StructMember::IndexAccess(access) => {
            let range = access
                .span
                .and_then(|span| crate::protocol::range(db, span))?;
            Some(DocumentSymbol {
                name: "[index]".to_owned(),
                kind: DocumentSymbolKind::Function,
                range,
                selection_range: range,
                detail: None,
                children: Vec::new(),
            })
        }
    }
}

const fn item_kind(kind: ItemKind) -> DocumentSymbolKind {
    match kind {
        ItemKind::CallableDecl => DocumentSymbolKind::Function,
        ItemKind::Struct => DocumentSymbolKind::Struct,
        ItemKind::Enum | ItemKind::Tag => DocumentSymbolKind::Enum,
        ItemKind::Import => DocumentSymbolKind::Module,
        ItemKind::Let => DocumentSymbolKind::Value,
        ItemKind::Expr => DocumentSymbolKind::Value,
    }
}
