use tune_syntax::{CstNode, SyntaxKind};

use crate::AstNode;

use super::text::first_string_text;

#[derive(Debug, Clone, Copy)]
pub struct ImportDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for ImportDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::ImportDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> ImportDecl<'tree> {
    #[must_use]
    pub fn path(self, source: &str) -> Option<&str> {
        first_string_text(self.node, source)
            .and_then(|text| text.strip_prefix('"'))
            .and_then(|text| text.strip_suffix('"'))
    }
}
