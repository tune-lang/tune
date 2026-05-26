use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::text::direct_ident_text;
use super::{Comment, Shape};

#[derive(Debug, Clone, Copy)]
pub struct StructDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for StructDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::StructDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> StructDecl<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }

    #[must_use]
    pub fn fields(self) -> Vec<DocumentedField<'tree>> {
        documented_fields(self.node)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FieldDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for FieldDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::FieldDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> FieldDecl<'tree> {
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
}

#[derive(Debug, Clone)]
pub struct DocumentedField<'tree> {
    pub field: FieldDecl<'tree>,
    pub docs: Vec<Comment>,
}

impl DocumentedField<'_> {
    #[must_use]
    pub fn doc_text(&self, source: &str) -> Option<String> {
        let lines = self
            .docs
            .iter()
            .filter_map(|comment| comment.doc_text(source))
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>();

        (!lines.is_empty()).then(|| lines.join("\n"))
    }
}

pub(super) fn documented_fields(node: &CstNode) -> Vec<DocumentedField<'_>> {
    let mut fields = Vec::new();
    let mut pending_docs = Vec::new();

    for child in &node.children {
        match child {
            CstElement::Token(token) => {
                if let Some(comment) = Comment::cast(*token) {
                    pending_docs.push(comment);
                } else if token.kind != TokenKind::Whitespace {
                    pending_docs.clear();
                }
            }
            CstElement::Node(node) => {
                if let Some(field) = FieldDecl::cast(node) {
                    fields.push(DocumentedField {
                        field,
                        docs: core::mem::take(&mut pending_docs),
                    });
                } else {
                    pending_docs.clear();
                }
            }
        }
    }

    fields
}
