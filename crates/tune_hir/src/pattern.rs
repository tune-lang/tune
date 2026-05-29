use crate::ExprId;
use crate::shape::ShapeExpr;
use tune_diagnostics::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub id: ExprId,
    pub span: Option<Span>,
    pub kind: PatternKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternKind {
    Hole,
    Binding(String),
    None,
    Unit,
    Tuple(Vec<Pattern>),
    Variant { name: String, args: Vec<Pattern> },
    StructuralShape(Vec<StructuralRequirement>),
    Else,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralRequirement {
    pub id: ExprId,
    pub span: Option<Span>,
    pub kind: StructuralRequirementKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralRequirementKind {
    Field {
        name: String,
        shape: Option<ShapeExpr>,
    },
    Callable {
        name: String,
        params: Vec<ShapeExpr>,
        ret: Option<ShapeExpr>,
    },
}
