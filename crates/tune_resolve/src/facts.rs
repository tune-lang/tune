use tune_diagnostics::Span;
use tune_hir::HirId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerFactKind {
    Name,
    Doc,
    Params,
    Return,
    Module,
    Visibility,
    JsonInvoker,
    Fields,
    Variants,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerFact {
    pub owner: HirId,
    pub kind: CompilerFactKind,
    pub value: String,
    pub span: Option<Span>,
}
