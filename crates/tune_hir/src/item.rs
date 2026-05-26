use crate::HirId;
use crate::shape::ShapeExpr;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagApplication {
    pub name: String,
    pub span: Option<Span>,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: Option<String>,
    pub span: Option<Span>,
    pub shape: Option<ShapeExpr>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: HirId,
    pub name: Option<String>,
    pub kind: ItemKind,
    pub visibility: Visibility,
    pub span: Option<Span>,
    pub doc: Option<String>,
    pub tags: Vec<TagApplication>,
    pub params: Vec<Param>,
    pub shape: Option<ShapeExpr>,
}
