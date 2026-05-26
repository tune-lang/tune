use tune_syntax::{CstElement, CstNode, TokenKind};

#[must_use]
pub fn first_ident_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    first_token_text(node, source, TokenKind::Ident)
}

#[must_use]
pub fn first_string_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    first_token_text(node, source, TokenKind::StringLiteral)
}

fn first_token_text<'src>(node: &CstNode, source: &'src str, kind: TokenKind) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) if token.kind == kind => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(node) => first_token_text(node, source, kind),
        CstElement::Token(_) => None,
    })
}
