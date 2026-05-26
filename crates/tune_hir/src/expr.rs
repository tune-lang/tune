use crate::HirId;

#[derive(Debug, Clone)]
pub enum ExprKind {
    Missing,
    Literal,
    Name(String),
    Let,
    Assign,
    Call,
    Field,
    Index,
    StructLiteral,
    Tuple,
    If,
    Match,
    For,
    Loop,
    Spawn,
    Propagate, // postfix !
    Block,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub id: HirId,
    pub kind: ExprKind,
}
