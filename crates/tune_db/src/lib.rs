pub mod ids;
pub mod interner;
pub mod source;

pub use ids::*;
pub use interner::Interner;
pub use source::{SourceFile, SourceMap};

pub struct ModuleAnalysis {
    pub parsed: tune_syntax::Parsed,
    pub module: tune_hir::module::Module,
    pub resolved: tune_resolve::ResolvedModule,
}

impl ModuleAnalysis {
    #[must_use]
    pub fn diagnostics(&self) -> Vec<tune_diagnostics::Diagnostic> {
        self.parsed
            .diagnostics
            .iter()
            .chain(self.resolved.diagnostics.iter())
            .cloned()
            .collect()
    }
}

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
    pub fn analyze_file(&self, id: FileId) -> Option<ModuleAnalysis> {
        let source = self.source(id)?;
        let parsed = tune_syntax::parse_with_file(id, &source.text);
        let module = tune_hir::lower::lower_module(&source.text, &parsed.cst);
        let resolved = tune_resolve::resolve_module(&module);

        Some(ModuleAnalysis {
            parsed,
            module,
            resolved,
        })
    }

    #[must_use]
    pub const fn symbols(&self) -> &Interner {
        &self.symbols
    }
}
