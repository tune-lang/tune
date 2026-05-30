use std::collections::BTreeMap;

use tune_db::{FileId, TuneDb};
use tune_hir::item::{ItemKind, Visibility};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSymbolKind {
    Function,
    Type,
    Value,
    Module,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    pub name: String,
    pub path: String,
    pub file: FileId,
    pub kind: WorkspaceSymbolKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct WorkspaceIndex {
    symbols: Vec<WorkspaceSymbol>,
}

impl WorkspaceIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rebuild(&mut self, db: &TuneDb) {
        let mut symbols = BTreeMap::new();
        for source in db.sources().iter() {
            let Some(analysis) = db.analyze_file(source.id) else {
                continue;
            };
            for item in &analysis.module.items {
                if item.visibility != Visibility::Public || item.kind == ItemKind::Import {
                    continue;
                }
                let Some(name) = item.name.clone() else {
                    continue;
                };
                let symbol = WorkspaceSymbol {
                    name: name.clone(),
                    path: source.path.clone(),
                    file: source.id,
                    kind: symbol_kind(item.kind),
                    detail: crate::hover::hover_card(
                        db,
                        source.id,
                        tune_resolve::FactOwner::Item(item.id),
                    )
                    .and_then(|hover| hover.signature),
                    documentation: item.doc.clone(),
                };
                symbols
                    .entry((symbol.name.clone(), symbol.path.clone()))
                    .or_insert(symbol);
            }
        }
        self.symbols = symbols.into_values().collect();
    }

    pub fn exports_named<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a WorkspaceSymbol> + 'a {
        self.symbols
            .iter()
            .filter(move |symbol| symbol.name == name)
    }

    pub fn symbols(&self) -> &[WorkspaceSymbol] {
        &self.symbols
    }
}

const fn symbol_kind(kind: ItemKind) -> WorkspaceSymbolKind {
    match kind {
        ItemKind::CallableDecl => WorkspaceSymbolKind::Function,
        ItemKind::Struct | ItemKind::Enum | ItemKind::Tag => WorkspaceSymbolKind::Type,
        ItemKind::Import => WorkspaceSymbolKind::Module,
        ItemKind::Let => WorkspaceSymbolKind::Value,
        ItemKind::Expr => WorkspaceSymbolKind::Value,
    }
}
