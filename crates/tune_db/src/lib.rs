pub mod ids;
pub mod interner;
pub mod source;

pub use ids::*;
pub use interner::Interner;
pub use source::{SourceFile, SourceMap};

#[derive(Default)]
pub struct TuneDb {
    sources: SourceMap,
    symbols: Interner,
}

impl TuneDb {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: impl Into<String>, text: impl Into<String>) -> Option<FileId> {
        self.sources.add_file(path, text)
    }

    #[must_use]
    pub const fn sources(&self) -> &SourceMap {
        &self.sources
    }

    #[must_use]
    pub fn source(&self, id: FileId) -> Option<&SourceFile> {
        self.sources.get(id)
    }

    pub fn intern(&mut self, text: &str) -> Option<SymbolId> {
        self.symbols.intern(text)
    }

    #[must_use]
    pub fn symbol(&self, id: SymbolId) -> Option<&str> {
        self.symbols.resolve(id)
    }

    #[must_use]
    pub const fn symbols(&self) -> &Interner {
        &self.symbols
    }
}
