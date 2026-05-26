use tune_diagnostics::Span;

#[derive(Debug, Clone)]
pub struct Pattern {
    pub span: Option<Span>,
    pub kind: PatternKind,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    Hole,
    Binding(String),
    Unit,
    Tuple(Vec<Pattern>),
    Variant { name: String, args: Vec<Pattern> },
    StructuralShape,
    Else,
}
