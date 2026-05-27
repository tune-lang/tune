use crate::ExprId;
use tune_diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub id: ExprId,
    pub span: Option<Span>,
    pub kind: PatternKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternKind {
    Hole,
    Binding(String),
    Unit,
    Tuple(Vec<Pattern>),
    Variant { name: String, args: Vec<Pattern> },
    StructuralShape,
    Else,
}
