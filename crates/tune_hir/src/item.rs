use crate::HirId;
use tune_diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Let,
    CallableDecl,
    Struct,
    Enum,
    Tag,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Private,
    Public,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: HirId,
    pub name: Option<String>,
    pub kind: ItemKind,
    pub visibility: Visibility,
    pub span: Option<Span>,
}
