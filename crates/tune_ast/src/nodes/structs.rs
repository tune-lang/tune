use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::text::direct_ident_text;
use super::{Comment, Expr, ParamList, Shape};

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
    pub fn type_params(self) -> Vec<TypeParamDecl<'tree>> {
        type_params(self.node)
    }

    #[must_use]
    pub fn fields(self) -> Vec<DocumentedField<'tree>> {
        self.members()
            .into_iter()
            .filter_map(|member| match member.member {
                StructMember::Field(field) => Some(DocumentedField {
                    field,
                    docs: member.docs,
                }),
                StructMember::Callable(_)
                | StructMember::SequenceMaterializer(_)
                | StructMember::IndexAccess(_) => None,
            })
            .collect()
    }

    #[must_use]
    pub fn members(self) -> Vec<DocumentedStructMember<'tree>> {
        documented_members(self.node)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TypeParamDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for TypeParamDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::TypeParam;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> TypeParamDecl<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
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

#[derive(Debug, Clone, Copy)]
pub struct MemberCallableDecl<'tree> {
    node: &'tree CstNode,
}

#[derive(Debug, Clone, Copy)]
pub struct SequenceMaterializerDecl<'tree> {
    node: &'tree CstNode,
}

#[derive(Debug, Clone, Copy)]
pub struct IndexAccessDecl<'tree> {
    node: &'tree CstNode,
}

macro_rules! member_node {
    ($name:ident, $kind:expr) => {
        impl<'tree> AstNode<'tree> for $name<'tree> {
            const KIND: SyntaxKind = $kind;

            fn cast(node: &'tree CstNode) -> Option<Self> {
                (node.kind == Self::KIND).then_some(Self { node })
            }

            fn syntax(&self) -> &'tree CstNode {
                self.node
            }
        }
    };
}

member_node!(MemberCallableDecl, SyntaxKind::MemberCallableDecl);
member_node!(
    SequenceMaterializerDecl,
    SyntaxKind::SequenceMaterializerDecl
);
member_node!(IndexAccessDecl, SyntaxKind::IndexAccessDecl);

impl<'tree> MemberCallableDecl<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }

    #[must_use]
    pub fn params(self) -> Option<ParamList<'tree>> {
        self.node.children.iter().find_map(|child| match child {
            CstElement::Node(node) => ParamList::cast(node),
            CstElement::Token(_) => None,
        })
    }

    #[must_use]
    pub fn shape_annotation(self) -> Option<Shape<'tree>> {
        first_shape(self.node)
    }

    #[must_use]
    pub fn body_expr(self) -> Option<Expr<'tree>> {
        first_expr(self.node)
    }
}

impl<'tree> SequenceMaterializerDecl<'tree> {
    #[must_use]
    pub fn param_name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }

    #[must_use]
    pub fn body_expr(self) -> Option<Expr<'tree>> {
        first_expr(self.node)
    }
}

impl<'tree> IndexAccessDecl<'tree> {
    #[must_use]
    pub fn receiver_name(self, source: &str) -> Option<&str> {
        direct_ident_text_at(self.node, source, 0)
    }

    #[must_use]
    pub fn index_param_name(self, source: &str) -> Option<&str> {
        direct_ident_text_at(self.node, source, 1)
    }

    #[must_use]
    pub fn shapes(self) -> Vec<Shape<'tree>> {
        self.node
            .children
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(node) => Shape::cast(node),
                CstElement::Token(_) => None,
            })
            .collect()
    }

    #[must_use]
    pub fn body_expr(self) -> Option<Expr<'tree>> {
        first_expr(self.node)
    }
}

#[derive(Debug, Clone)]
pub struct DocumentedField<'tree> {
    pub field: FieldDecl<'tree>,
    pub docs: Vec<Comment>,
}

#[derive(Debug, Clone, Copy)]
pub enum StructMember<'tree> {
    Field(FieldDecl<'tree>),
    Callable(MemberCallableDecl<'tree>),
    SequenceMaterializer(SequenceMaterializerDecl<'tree>),
    IndexAccess(IndexAccessDecl<'tree>),
}

#[derive(Debug, Clone)]
pub struct DocumentedStructMember<'tree> {
    pub member: StructMember<'tree>,
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

impl DocumentedStructMember<'_> {
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

pub(super) fn documented_members(node: &CstNode) -> Vec<DocumentedStructMember<'_>> {
    let mut members = Vec::new();
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
                if let Some(member) = struct_member(node) {
                    members.push(DocumentedStructMember {
                        member,
                        docs: core::mem::take(&mut pending_docs),
                    });
                } else {
                    pending_docs.clear();
                }
            }
        }
    }

    members
}

pub(super) fn documented_fields(node: &CstNode) -> Vec<DocumentedField<'_>> {
    documented_members(node)
        .into_iter()
        .filter_map(|member| match member.member {
            StructMember::Field(field) => Some(DocumentedField {
                field,
                docs: member.docs,
            }),
            StructMember::Callable(_)
            | StructMember::SequenceMaterializer(_)
            | StructMember::IndexAccess(_) => None,
        })
        .collect()
}

fn struct_member(node: &CstNode) -> Option<StructMember<'_>> {
    match node.kind {
        SyntaxKind::FieldDecl => FieldDecl::cast(node).map(StructMember::Field),
        SyntaxKind::MemberCallableDecl => {
            MemberCallableDecl::cast(node).map(StructMember::Callable)
        }
        SyntaxKind::SequenceMaterializerDecl => {
            SequenceMaterializerDecl::cast(node).map(StructMember::SequenceMaterializer)
        }
        SyntaxKind::IndexAccessDecl => IndexAccessDecl::cast(node).map(StructMember::IndexAccess),
        _ => None,
    }
}

fn first_shape<'tree>(node: &'tree CstNode) -> Option<Shape<'tree>> {
    node.children.iter().find_map(|child| match child {
        CstElement::Node(node) => Shape::cast(node),
        CstElement::Token(_) => None,
    })
}

fn first_expr<'tree>(node: &'tree CstNode) -> Option<Expr<'tree>> {
    node.children.iter().find_map(|child| match child {
        CstElement::Node(node) => Expr::cast(node),
        CstElement::Token(_) => None,
    })
}

fn direct_ident_text_at<'src>(
    node: &CstNode,
    source: &'src str,
    index: usize,
) -> Option<&'src str> {
    node.children
        .iter()
        .filter_map(|child| match child {
            CstElement::Token(token) if token.kind == TokenKind::Ident => {
                let start = token.span.start.get() as usize;
                let end = token.span.end.get() as usize;
                source.get(start..end)
            }
            CstElement::Node(_) | CstElement::Token(_) => None,
        })
        .nth(index)
}

pub(super) fn type_params(node: &CstNode) -> Vec<TypeParamDecl<'_>> {
    node.children
        .iter()
        .find_map(|child| match child {
            CstElement::Node(node) if node.kind == SyntaxKind::TypeParamList => Some(node),
            CstElement::Node(_) | CstElement::Token(_) => None,
        })
        .map_or_else(Vec::new, |node| {
            node.children
                .iter()
                .filter_map(|child| match child {
                    CstElement::Node(node) => TypeParamDecl::cast(node),
                    CstElement::Token(_) => None,
                })
                .collect()
        })
}
