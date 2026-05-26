use tune_syntax::{CstNode, SyntaxKind};

use crate::AstNode;

use super::text::first_ident_text;

#[derive(Debug, Clone, Copy)]
pub struct EnumDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for EnumDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::EnumDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> EnumDecl<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        first_ident_text(self.node, source)
    }
}
