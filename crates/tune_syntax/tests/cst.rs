use tune_syntax::{CstBuilder, CstElement, SyntaxKind, Token, TokenKind, parse};

#[test]
fn parse_preserves_tokens_and_trivia_in_root() {
    let root = parse("let x = 1 -- trailing");

    assert_eq!(root.kind, SyntaxKind::Root);
    assert_eq!(root.children.len(), 10);
    assert_eq!(token_kind(&root.children[0]), Some(TokenKind::KeywordLet));
    assert_eq!(token_kind(&root.children[1]), Some(TokenKind::Whitespace));
    assert_eq!(token_kind(&root.children[2]), Some(TokenKind::Ident));
    assert_eq!(token_kind(&root.children[4]), Some(TokenKind::Equal));
    assert_eq!(token_kind(&root.children[6]), Some(TokenKind::IntLiteral));
    assert_eq!(token_kind(&root.children[8]), Some(TokenKind::LineComment));
    assert_eq!(token_kind(&root.children[9]), Some(TokenKind::Eof));
}

#[test]
fn parse_root_span_covers_non_empty_source() {
    let root = parse("let x = 1");
    let spans = root.span.into_iter().collect::<Vec<_>>();

    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].start.get(), 0);
    assert_eq!(spans[0].end.get(), 9);
}

#[test]
fn builder_computes_nested_node_spans() {
    let file = tune_diagnostics::FileId(4);
    let first = Token::new(
        TokenKind::KeywordLet,
        tune_diagnostics::Span::new(
            file,
            tune_diagnostics::ByteOffset::new(3),
            tune_diagnostics::ByteOffset::new(6),
        ),
    );
    let second = Token::new(
        TokenKind::Ident,
        tune_diagnostics::Span::new(
            file,
            tune_diagnostics::ByteOffset::new(7),
            tune_diagnostics::ByteOffset::new(11),
        ),
    );

    let mut builder = CstBuilder::new(SyntaxKind::Root);
    builder.start_node(SyntaxKind::LetDecl);
    builder.token(first);
    builder.token(second);
    builder.finish_node();
    let root = builder.finish();

    let let_decl_spans = root
        .children
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(node) if node.kind == SyntaxKind::LetDecl => node.span,
            CstElement::Node(_) | CstElement::Token(_) => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(let_decl_spans.len(), 1);
    assert_eq!(let_decl_spans[0].start.get(), 3);
    assert_eq!(let_decl_spans[0].end.get(), 11);
}

fn token_kind(element: &CstElement) -> Option<TokenKind> {
    match element {
        CstElement::Token(token) => Some(token.kind),
        CstElement::Node(_) => None,
    }
}
