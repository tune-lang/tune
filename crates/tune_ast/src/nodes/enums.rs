use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::structs::type_params;
use super::text::{direct_ident_text, first_variant_name_text};
use super::{Comment, Shape};

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
        direct_ident_text(self.node, source)
    }

    #[must_use]
    pub fn type_params(self) -> Vec<super::structs::TypeParamDecl<'tree>> {
        type_params(self.node)
    }

    #[must_use]
    pub fn variants(self) -> Vec<DocumentedVariant<'tree>> {
        let mut variants = Vec::new();
        let mut pending_docs = Vec::new();

        for child in &self.node.children {
            match child {
                CstElement::Token(token) => {
                    if let Some(comment) = Comment::cast(*token) {
                        pending_docs.push(comment);
                    } else if token.kind != TokenKind::Whitespace {
                        pending_docs.clear();
                    }
                }
                CstElement::Node(node) => {
                    if let Some(variant) = VariantDecl::cast(node) {
                        variants.push(DocumentedVariant {
                            variant,
                            docs: core::mem::take(&mut pending_docs),
                        });
                    } else {
                        pending_docs.clear();
                    }
                }
            }
        }

        variants
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VariantDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for VariantDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::VariantDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> VariantDecl<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        first_variant_name_text(self.node, source)
    }

    #[must_use]
    pub fn payload_shapes(self) -> Vec<Shape<'tree>> {
        let mut shapes = Vec::new();
        collect_shapes(self.node, &mut shapes);
        shapes
    }
}

#[derive(Debug, Clone)]
pub struct DocumentedVariant<'tree> {
    pub variant: VariantDecl<'tree>,
    pub docs: Vec<Comment>,
}

impl DocumentedVariant<'_> {
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

fn collect_shapes<'tree>(node: &'tree CstNode, shapes: &mut Vec<Shape<'tree>>) {
    for child in &node.children {
        if let CstElement::Node(node) = child {
            if let Some(shape) = Shape::cast(node) {
                shapes.push(shape);
            } else {
                collect_shapes(node, shapes);
            }
        }
    }
}
