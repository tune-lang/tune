use crate::ExprId;
use crate::pattern::Pattern;
use tune_diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiteralKind {
    Int(String),
    Float(String),
    String(String),
    Bool(bool),
    None,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Missing,
    Literal(LiteralKind),
    Sequence(Vec<Expr>),
    Name(String),
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Field {
        base: Box<Expr>,
        name: Option<String>,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Spawn(Box<Expr>),
    Propagate(Box<Expr>),
    For {
        pattern: Pattern,
        iterable: Box<Expr>,
        body: Box<Expr>,
    },
    Block(Vec<Expr>),
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub id: ExprId,
    pub span: Option<Span>,
    pub kind: ExprKind,
}
