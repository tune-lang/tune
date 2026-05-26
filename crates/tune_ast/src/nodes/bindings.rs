use tune_syntax::{CstElement, CstNode, SyntaxKind};

use crate::AstNode;

use super::text::direct_ident_text;
use super::{ParamList, Shape};

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
        direct_ident_text(self.node, source)
    }

    #[must_use]
    pub fn shape_annotation(self) -> Option<Shape<'tree>> {
        self.node.children.iter().find_map(|child| match child {
            CstElement::Node(node) => Shape::cast(node),
            CstElement::Token(_) => None,
        })
    }

    #[must_use]
    pub fn params(self) -> Option<ParamList<'tree>> {
        self.node.children.iter().find_map(|child| match child {
            CstElement::Node(node) => ParamList::cast(node),
            CstElement::Token(_) => None,
        })
    }
}
