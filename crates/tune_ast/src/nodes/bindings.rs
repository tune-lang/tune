use tune_syntax::{CstNode, SyntaxKind};

use crate::AstNode;

use super::text::first_ident_text;

#[derive(Debug, Clone, Copy)]
pub struct LetDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for LetDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::LetDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        matches!(node.kind, SyntaxKind::LetDecl | SyntaxKind::CallableDecl).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> LetDecl<'tree> {
    #[must_use]
    pub fn is_callable_decl(self) -> bool {
        self.node.kind == SyntaxKind::CallableDecl
    }

    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        first_ident_text(self.node, source)
    }
}
