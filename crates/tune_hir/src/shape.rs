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
    Generic {
        name: String,
        args: Vec<ShapeExpr>,
    },
    Sequence(Box<ShapeExpr>),
    Tuple(Vec<ShapeExpr>),
    Optional(Box<ShapeExpr>),
    Union(Vec<ShapeExpr>),
    Structural(Vec<StructuralShapeRequirement>),
    Callable {
        params: Vec<ShapeExpr>,
        ret: Box<ShapeExpr>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralShapeRequirement {
    pub name: String,
    pub span: Option<Span>,
    pub kind: StructuralShapeRequirementKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralShapeRequirementKind {
    Field {
        shape: Option<ShapeExpr>,
    },
    Callable {
        params: Vec<ShapeExpr>,
        ret: Option<ShapeExpr>,
    },
}
