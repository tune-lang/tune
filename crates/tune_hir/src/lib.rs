pub mod expr;
pub mod item;
pub mod lower;
pub mod module;
pub mod pattern;
pub mod shape;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HirId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberKind {
    Param,
    Field,
    Variant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemberId {
    pub owner: HirId,
    pub kind: MemberKind,
    pub index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);
