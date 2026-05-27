use tune_diagnostics::Span;
use tune_hir::{ExprId, HirId, MemberId};

use crate::prelude::VariantId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    Let,
    Pattern,
    CallableParam,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalBinding {
    pub id: LocalId,
    pub owner: HirId,
    pub kind: LocalKind,
    pub name: String,
    pub expr: Option<ExprId>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameTarget {
    TopLevel(HirId),
    Variant(VariantId),
    Param(MemberId),
    Local(LocalId),
    SelfValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NameRef {
    pub expr: ExprId,
    pub target: NameTarget,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantPatternRef {
    pub variant: VariantId,
    pub span: Option<Span>,
}
