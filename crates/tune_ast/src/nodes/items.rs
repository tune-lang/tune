use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::{Comment, EnumDecl, ImportDecl, LetDecl, StructDecl, TagApplication, TagDecl};

#[derive(Debug, Clone, Copy)]
pub struct Root<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for Root<'tree> {
    const KIND: SyntaxKind = SyntaxKind::Root;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> Root<'tree> {
    pub fn items(self) -> impl Iterator<Item = Item<'tree>> {
        self.node.children.iter().filter_map(|child| match child {
            CstElement::Node(node) => Item::cast(node),
            CstElement::Token(_) => None,
        })
    }

    #[must_use]
    pub fn documented_items(self) -> Vec<DocumentedItem<'tree>> {
        let mut documented = Vec::new();
        let mut pending_docs = Vec::new();
        let mut pending_tags = Vec::new();

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
                    if let Some(item) = Item::cast(node) {
                        documented.push(DocumentedItem {
                            item,
                            docs: core::mem::take(&mut pending_docs),
                            tags: core::mem::take(&mut pending_tags),
                        });
                    } else if let Some(tag) = TagApplication::cast(node) {
                        pending_tags.push(tag);
                    } else {
                        pending_docs.clear();
                        pending_tags.clear();
                    }
                }
            }
        }

        documented
    }
}

#[derive(Debug, Clone)]
pub struct DocumentedItem<'tree> {
    pub item: Item<'tree>,
    pub docs: Vec<Comment>,
    pub tags: Vec<TagApplication<'tree>>,
}

impl<'tree> DocumentedItem<'tree> {
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

#[derive(Debug, Clone, Copy)]
pub struct PubDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for PubDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::PubDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> PubDecl<'tree> {
    #[must_use]
    pub fn item(self) -> Option<Item<'tree>> {
        self.node.children.iter().find_map(|child| match child {
            CstElement::Node(node) => Item::cast(node),
            CstElement::Token(_) => None,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Item<'tree> {
    Import(ImportDecl<'tree>),
    Let(LetDecl<'tree>),
    Struct(StructDecl<'tree>),
    Enum(EnumDecl<'tree>),
    Tag(TagDecl<'tree>),
    Pub(PubDecl<'tree>),
}

impl<'tree> Item<'tree> {
    #[must_use]
    pub fn cast(node: &'tree CstNode) -> Option<Self> {
        match node.kind {
            SyntaxKind::ImportDecl => ImportDecl::cast(node).map(Self::Import),
            SyntaxKind::LetDecl | SyntaxKind::CallableDecl => LetDecl::cast(node).map(Self::Let),
            SyntaxKind::StructDecl => StructDecl::cast(node).map(Self::Struct),
            SyntaxKind::EnumDecl => EnumDecl::cast(node).map(Self::Enum),
            SyntaxKind::TagDecl => TagDecl::cast(node).map(Self::Tag),
            SyntaxKind::PubDecl => PubDecl::cast(node).map(Self::Pub),
            _ => None,
        }
    }

    #[must_use]
    pub fn syntax(self) -> &'tree CstNode {
        match self {
            Self::Import(node) => node.syntax(),
            Self::Let(node) => node.syntax(),
            Self::Struct(node) => node.syntax(),
            Self::Enum(node) => node.syntax(),
            Self::Tag(node) => node.syntax(),
            Self::Pub(node) => node.syntax(),
        }
    }
}
