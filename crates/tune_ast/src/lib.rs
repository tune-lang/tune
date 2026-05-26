pub mod nodes;

pub trait AstNode {
    fn kind_name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AstId(pub u32);
