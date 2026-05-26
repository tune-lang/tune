use tune_diagnostics::Span;
use tune_hir::item::Visibility;
use tune_hir::shape::ShapeExpr;
use tune_hir::{HirId, MemberId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerFactKind {
    Name,
    Doc,
    Params,
    Return,
    Module,
    Visibility,
    JsonInvoker,
    Fields,
    Variants,
    Tag,
    Shape,
    Payload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FactOwner {
    Item(HirId),
    Member(MemberId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerFactPayload {
    Name(String),
    Doc(String),
    Params(Vec<MemberId>),
    Return(ShapeExpr),
    Module(String),
    Visibility(Visibility),
    JsonInvoker(String),
    Fields(Vec<MemberId>),
    Variants(Vec<MemberId>),
    Tag(String),
    Shape(ShapeExpr),
    Payload(Vec<ShapeExpr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerFact {
    pub owner: FactOwner,
    pub payload: CompilerFactPayload,
    pub span: Option<Span>,
}

impl CompilerFact {
    #[must_use]
    pub fn kind(&self) -> CompilerFactKind {
        match &self.payload {
            CompilerFactPayload::Name(_) => CompilerFactKind::Name,
            CompilerFactPayload::Doc(_) => CompilerFactKind::Doc,
            CompilerFactPayload::Params(_) => CompilerFactKind::Params,
            CompilerFactPayload::Return(_) => CompilerFactKind::Return,
            CompilerFactPayload::Module(_) => CompilerFactKind::Module,
            CompilerFactPayload::Visibility(_) => CompilerFactKind::Visibility,
            CompilerFactPayload::JsonInvoker(_) => CompilerFactKind::JsonInvoker,
            CompilerFactPayload::Fields(_) => CompilerFactKind::Fields,
            CompilerFactPayload::Variants(_) => CompilerFactKind::Variants,
            CompilerFactPayload::Tag(_) => CompilerFactKind::Tag,
            CompilerFactPayload::Shape(_) => CompilerFactKind::Shape,
            CompilerFactPayload::Payload(_) => CompilerFactKind::Payload,
        }
    }
}
