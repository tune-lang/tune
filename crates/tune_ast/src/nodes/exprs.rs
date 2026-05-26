use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::text::direct_ident_text;

#[derive(Debug, Clone, Copy)]
pub enum Expr<'tree> {
    Missing(&'tree CstNode),
    Literal(LiteralExpr<'tree>),
    Name(NameExpr<'tree>),
    Call(CallExpr<'tree>),
    Field(FieldExpr<'tree>),
    Index(IndexExpr<'tree>),
    Propagate(PropagateExpr<'tree>),
    For(ForExpr<'tree>),
    Spawn(SpawnExpr<'tree>),
    Block(BlockExpr<'tree>),
}

impl<'tree> Expr<'tree> {
    #[must_use]
    pub fn cast(node: &'tree CstNode) -> Option<Self> {
        match node.kind {
            SyntaxKind::LiteralExpr => LiteralExpr::cast(node).map(Self::Literal),
            SyntaxKind::NameExpr => NameExpr::cast(node).map(Self::Name),
            SyntaxKind::CallExpr => CallExpr::cast(node).map(Self::Call),
            SyntaxKind::FieldExpr => FieldExpr::cast(node).map(Self::Field),
            SyntaxKind::IndexExpr => IndexExpr::cast(node).map(Self::Index),
            SyntaxKind::PropagateExpr => PropagateExpr::cast(node).map(Self::Propagate),
            SyntaxKind::ForExpr => ForExpr::cast(node).map(Self::For),
            SyntaxKind::SpawnExpr => SpawnExpr::cast(node).map(Self::Spawn),
            SyntaxKind::Block => BlockExpr::cast(node).map(Self::Block),
            SyntaxKind::Expr => Some(Self::Missing(node)),
            _ => None,
        }
    }

    #[must_use]
    pub fn syntax(self) -> &'tree CstNode {
        match self {
            Self::Missing(node) => node,
            Self::Literal(node) => node.syntax(),
            Self::Name(node) => node.syntax(),
            Self::Call(node) => node.syntax(),
            Self::Field(node) => node.syntax(),
            Self::Index(node) => node.syntax(),
            Self::Propagate(node) => node.syntax(),
            Self::For(node) => node.syntax(),
            Self::Spawn(node) => node.syntax(),
            Self::Block(node) => node.syntax(),
        }
    }

    #[must_use]
    pub fn child_exprs(self) -> Vec<Expr<'tree>> {
        child_exprs(self.syntax())
    }
}

macro_rules! expr_node {
    ($name:ident, $kind:expr) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'tree> {
            node: &'tree CstNode,
        }

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

expr_node!(LiteralExpr, SyntaxKind::LiteralExpr);
expr_node!(NameExpr, SyntaxKind::NameExpr);
expr_node!(CallExpr, SyntaxKind::CallExpr);
expr_node!(FieldExpr, SyntaxKind::FieldExpr);
expr_node!(IndexExpr, SyntaxKind::IndexExpr);
expr_node!(PropagateExpr, SyntaxKind::PropagateExpr);
expr_node!(ForExpr, SyntaxKind::ForExpr);
expr_node!(SpawnExpr, SyntaxKind::SpawnExpr);
expr_node!(BlockExpr, SyntaxKind::Block);

impl<'tree> LiteralExpr<'tree> {
    #[must_use]
    pub fn text(self, source: &str) -> Option<&str> {
        first_direct_token_text(self.node, source)
    }
}

impl<'tree> NameExpr<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source).or_else(|| direct_self_text(self.node, source))
    }
}

impl<'tree> FieldExpr<'tree> {
    #[must_use]
    pub fn field_name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }
}

impl<'tree> BlockExpr<'tree> {
    #[must_use]
    pub fn exprs(self) -> Vec<Expr<'tree>> {
        child_exprs(self.node)
    }
}

fn child_exprs(node: &CstNode) -> Vec<Expr<'_>> {
    node.children
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(node) => Expr::cast(node),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn first_direct_token_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(_) => None,
    })
}

fn direct_self_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) if token.kind == TokenKind::KeywordSelf => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(_) | CstElement::Token(_) => None,
    })
}
