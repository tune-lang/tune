use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::exprs::Expr;
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

#[derive(Debug, Clone, Copy)]
pub struct TagArg<'tree> {
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

impl<'tree> AstNode<'tree> for TagArg<'tree> {
    const KIND: SyntaxKind = SyntaxKind::TagArg;

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

    #[must_use]
    pub fn args(self) -> Vec<TagArg<'tree>> {
        self.node
            .children
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(node) if node.kind == SyntaxKind::TagArgList => {
                    Some(tag_args(node))
                }
                CstElement::Node(_) | CstElement::Token(_) => None,
            })
            .flatten()
            .collect()
    }
}

impl<'tree> TagArg<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        has_name_separator(self.node).then(|| direct_ident_text(self.node, source))?
    }

    #[must_use]
    pub fn value_expr(self) -> Option<Expr<'tree>> {
        self.node.children.iter().find_map(|child| match child {
            CstElement::Node(node) => Expr::cast(node),
            CstElement::Token(_) => None,
        })
    }
}

fn tag_args(node: &CstNode) -> Vec<TagArg<'_>> {
    node.children
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(node) => TagArg::cast(node),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn has_name_separator(node: &CstNode) -> bool {
    node.children.iter().any(|child| match child {
        CstElement::Token(token) => matches!(token.kind, TokenKind::Colon | TokenKind::Equal),
        CstElement::Node(_) => false,
    })
}
