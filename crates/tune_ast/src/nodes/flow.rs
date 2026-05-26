#[derive(Debug, Clone)]
pub enum FlowKind {
    If,
    Match,
    For,
    While,
    Loop,
}
