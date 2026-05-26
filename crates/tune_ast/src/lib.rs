pub mod nodes;

use tune_syntax::{CstNode, SyntaxKind};

pub trait AstNode<'tree>: Sized {
    const KIND: SyntaxKind;

    fn cast(node: &'tree CstNode) -> Option<Self>;

    fn syntax(&self) -> &'tree CstNode;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AstId(pub u32);
