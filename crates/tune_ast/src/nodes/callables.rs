use tune_syntax::{CstElement, CstNode, SyntaxKind};

use crate::AstNode;

use super::Shape;
use super::text::direct_ident_text;

#[derive(Debug, Clone)]
pub struct CallableHead {
    pub name: Option<String>, // None represents `_` anonymous callable.
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ParamList<'tree> {
    node: &'tree CstNode,
}

#[derive(Debug, Clone, Copy)]
pub struct CallableParam<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for ParamList<'tree> {
    const KIND: SyntaxKind = SyntaxKind::ParamList;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> AstNode<'tree> for CallableParam<'tree> {
    const KIND: SyntaxKind = SyntaxKind::Param;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> ParamList<'tree> {
    pub fn params(self) -> impl Iterator<Item = CallableParam<'tree>> {
        self.node.children.iter().filter_map(|child| match child {
            CstElement::Node(node) => CallableParam::cast(node),
            CstElement::Token(_) => None,
        })
    }
}

impl<'tree> CallableParam<'tree> {
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
