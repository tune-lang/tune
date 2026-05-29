use crate::expr::Expr;
use crate::shape::ShapeExpr;
use crate::{HirId, MemberId};
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
pub enum ExternalItem {
    HostFunction {
        symbol: ExternalSymbolId,
        task_safe: bool,
    },
    ModuleNamespace {
        members: Vec<ModuleNamespaceMember>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExternalSymbolId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleNamespaceMember {
    pub name: String,
    pub item: HirId,
}

#[derive(Debug, Clone)]
pub struct TagApplication {
    pub name: String,
    pub span: Option<Span>,
    pub args: Vec<TagArg>,
}

#[derive(Debug, Clone)]
pub struct TagArg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSpec {
    pub path: String,
    pub selector: ImportSelector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSelector {
    Module,
    Member(String),
    Members(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub id: MemberId,
    pub name: Option<String>,
    pub span: Option<Span>,
    pub shape: Option<ShapeExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    pub id: MemberId,
    pub name: Option<String>,
    pub span: Option<Span>,
    pub constraint: Option<ShapeExpr>,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub id: MemberId,
    pub name: Option<String>,
    pub span: Option<Span>,
    pub doc: Option<String>,
    pub shape: Option<ShapeExpr>,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone)]
pub enum StructMember {
    Field(Field),
    Callable(CallableMember),
    SequenceMaterializer(SequenceMaterializer),
    IndexAccess(IndexAccess),
}

#[derive(Debug, Clone)]
pub struct CallableMember {
    pub id: MemberId,
    pub name: Option<String>,
    pub span: Option<Span>,
    pub doc: Option<String>,
    pub params: Vec<Param>,
    pub shape: Option<ShapeExpr>,
    pub body: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct SequenceMaterializer {
    pub id: MemberId,
    pub param_name: Option<String>,
    pub span: Option<Span>,
    pub doc: Option<String>,
    pub body: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct IndexAccess {
    pub id: MemberId,
    pub index_param_id: MemberId,
    pub receiver_name: Option<String>,
    pub index_param_name: Option<String>,
    pub span: Option<Span>,
    pub doc: Option<String>,
    pub index_shape: Option<ShapeExpr>,
    pub result_shape: Option<ShapeExpr>,
    pub body: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub id: MemberId,
    pub name: Option<String>,
    pub span: Option<Span>,
    pub doc: Option<String>,
    pub payload: Vec<ShapeExpr>,
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
    pub import: Option<ImportSpec>,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub struct_members: Vec<StructMember>,
    pub fields: Vec<Field>,
    pub variants: Vec<Variant>,
    pub shape: Option<ShapeExpr>,
    pub body: Option<Expr>,
    pub external: Option<ExternalItem>,
}
