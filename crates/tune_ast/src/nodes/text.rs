use tune_syntax::{CstElement, CstNode, TokenKind};

#[must_use]
pub fn direct_ident_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    first_matching_direct_token_text(node, source, |kind| kind == TokenKind::Ident)
}

#[must_use]
pub fn first_variant_name_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    first_matching_direct_token_text(node, source, |kind| {
        matches!(
            kind,
            TokenKind::Ident | TokenKind::KeywordOk | TokenKind::KeywordError
        )
    })
}

#[must_use]
pub fn first_string_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    first_token_text(node, source, TokenKind::StringLiteral)
}

fn first_token_text<'src>(node: &CstNode, source: &'src str, kind: TokenKind) -> Option<&'src str> {
    first_matching_token_text(node, source, |token_kind| token_kind == kind)
}

fn first_matching_direct_token_text<'src>(
    node: &CstNode,
    source: &'src str,
    matches: impl Copy + Fn(TokenKind) -> bool,
) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) if matches(token.kind) => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(_) | CstElement::Token(_) => None,
    })
}

fn first_matching_token_text<'src>(
    node: &CstNode,
    source: &'src str,
    matches: impl Copy + Fn(TokenKind) -> bool,
) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) if matches(token.kind) => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(node) => first_matching_token_text(node, source, matches),
        CstElement::Token(_) => None,
    })
}
