use crate::ExprId;
use crate::pattern::Pattern;
use crate::shape::ShapeExpr;
use tune_diagnostics::Span;

#[derive(Debug, Clone)]
pub enum LiteralKind {
    Int(String),
    Float(String),
    String(StringLiteral),
    Bool(bool),
    None,
}

#[derive(Debug, Clone)]
pub struct StringLiteral {
    pub parts: Vec<StringPart>,
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Text(String),
    Interpolation(Box<Expr>),
}

impl StringLiteral {
    #[must_use]
    pub fn plain_text(&self) -> Option<String> {
        let mut text = String::new();
        for part in &self.parts {
            match part {
                StringPart::Text(part) => text.push_str(part),
                StringPart::Interpolation(_) => return None,
            }
        }
        Some(text)
    }
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Missing,
    Literal(LiteralKind),
    Tuple(Vec<Expr>),
    Sequence(Vec<Expr>),
    Struct {
        name: String,
        fields: Vec<StructFieldInit>,
    },
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
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Spawn(Box<Expr>),
    Propagate(Box<Expr>),
    If {
        branches: Vec<IfBranch>,
        else_branch: Option<Box<Expr>>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    While {
        condition: Box<Expr>,
        body: Box<Expr>,
    },
    Loop(Box<Expr>),
    Break,
    Continue,
    Return(Option<Box<Expr>>),
    Panic(Vec<Expr>),
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

#[derive(Debug, Clone)]
pub struct StructFieldInit {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct IfBranch {
    pub condition: Expr,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Invert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    BitOr,
    BitXor,
    BitAnd,
    ShiftLeft,
    ShiftRight,
    RangeExclusive,
    RangeInclusive,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}
