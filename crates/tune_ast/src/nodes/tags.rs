use tune_syntax::{CstNode, SyntaxKind};

use crate::AstNode;

use super::structs::{DocumentedField, documented_fields};
use super::text::direct_ident_text;

#[derive(Debug, Clone, Copy)]
pub struct TagDecl<'tree> {
    node: &'tree CstNode,
}

#[derive(Debug, Clone, Copy)]
pub struct TagApplication<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for TagDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::TagDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> AstNode<'tree> for TagApplication<'tree> {
    const KIND: SyntaxKind = SyntaxKind::TagApplication;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> TagDecl<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }

    #[must_use]
    pub fn fields(self) -> Vec<DocumentedField<'tree>> {
        documented_fields(self.node)
    }
}

impl<'tree> TagApplication<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }
}
