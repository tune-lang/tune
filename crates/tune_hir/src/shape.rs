use tune_diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeExpr {
    pub kind: ShapeExprKind,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeExprKind {
    Missing,
    Named(String),
    Sequence(Box<ShapeExpr>),
    Tuple(Vec<ShapeExpr>),
    Optional(Box<ShapeExpr>),
    Union(Vec<ShapeExpr>),
    Callable {
        params: Vec<ShapeExpr>,
        ret: Box<ShapeExpr>,
    },
}
