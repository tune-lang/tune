use tune_syntax::{CstElement, CstNode, TokenKind};

#[must_use]
pub fn first_ident_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
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
