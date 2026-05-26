use crate::ExprId;
use crate::pattern::Pattern;
use crate::shape::ShapeExpr;
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
    CallableValue {
        params: Vec<ExprParam>,
        body: Box<Expr>,
    },
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
    Let {
        name: Option<String>,
        shape: Option<ShapeExpr>,
        value: Option<Box<Expr>>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Spawn(Box<Expr>),
    Propagate(Box<Expr>),
    Return(Option<Box<Expr>>),
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

#[derive(Debug, Clone)]
pub struct ExprParam {
    pub name: Option<String>,
    pub span: Option<Span>,
    pub shape: Option<ShapeExpr>,
}
