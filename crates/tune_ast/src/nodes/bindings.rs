use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::text::direct_ident_text;
use super::{Comment, Expr, ParamList, Shape, TypeParamDecl};

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
    pub fn type_params(self) -> Vec<TypeParamDecl<'tree>> {
        super::structs::type_params(self.node)
    }

    #[must_use]
    pub fn params(self) -> Option<ParamList<'tree>> {
        self.node.children.iter().find_map(|child| match child {
            CstElement::Node(node) => ParamList::cast(node),
            CstElement::Token(_) => None,
        })
    }

    #[must_use]
    pub fn body_expr(self) -> Option<Expr<'tree>> {
        let mut past_equals = false;
        self.node.children.iter().find_map(|child| match child {
            CstElement::Token(token) if token.kind == TokenKind::Equal => {
                past_equals = true;
                None
            }
            CstElement::Node(node) if past_equals => Expr::cast(node),
            CstElement::Node(_) | CstElement::Token(_) => None,
        })
    }

    #[must_use]
    pub fn signature_docs(self) -> Vec<Comment> {
        if !self.is_callable_decl() {
            return Vec::new();
        }

        let mut docs = Vec::new();
        let mut past_signature_start = false;

        for child in &self.node.children {
            match child {
                CstElement::Node(node) => {
                    if ParamList::cast(node).is_some() || Shape::cast(node).is_some() {
                        past_signature_start = true;
                    }
                }
                CstElement::Token(token) if token.kind == TokenKind::Equal => break,
                CstElement::Token(token) if past_signature_start => {
                    if let Some(comment) = Comment::cast(*token) {
                        docs.push(comment);
                    }
                }
                CstElement::Token(_) => {}
            }
        }

        docs
    }
}
