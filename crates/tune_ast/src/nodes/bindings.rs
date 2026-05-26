use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

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
        first_ident_text(self.node, source)
    }
}

fn first_ident_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) if token.kind == TokenKind::Ident => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(node) => first_ident_text(node, source),
        CstElement::Token(_) => None,
    })
}
