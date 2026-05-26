use tune_syntax::{CstBuilder, CstElement, SyntaxKind, Token, TokenKind, parse};

#[test]
fn parse_preserves_tokens_and_trivia_in_root() {
    let parsed = parse("let x = 1 -- trailing");
    let token_kinds = all_token_kinds(&parsed.cst);

    assert_eq!(parsed.cst.kind, SyntaxKind::Root);
    assert_eq!(
        token_kinds,
        [
            TokenKind::KeywordLet,
            TokenKind::Whitespace,
            TokenKind::Ident,
            TokenKind::Whitespace,
            TokenKind::Equal,
            TokenKind::Whitespace,
            TokenKind::IntLiteral,
            TokenKind::Whitespace,
            TokenKind::LineComment,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn parse_root_span_covers_non_empty_source() {
    let root = parse("let x = 1").cst;
    let spans = root.span.into_iter().collect::<Vec<_>>();

    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].start.get(), 0);
    assert_eq!(spans[0].end.get(), 9);
}

#[test]
fn parse_keeps_lexer_diagnostics() {
    let parsed = parse("\"unterminated");

    let messages = parsed
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.title.as_str())
        .collect::<Vec<_>>();

    assert!(messages.contains(&"unterminated string literal"));
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

#[test]
fn builder_can_wrap_children_from_checkpoint() {
    let file = tune_diagnostics::FileId(2);
    let name = Token::new(
        TokenKind::Ident,
        tune_diagnostics::Span::new(
            file,
            tune_diagnostics::ByteOffset::new(0),
            tune_diagnostics::ByteOffset::new(6),
        ),
    );
    let question = Token::new(
        TokenKind::Question,
        tune_diagnostics::Span::new(
            file,
            tune_diagnostics::ByteOffset::new(6),
            tune_diagnostics::ByteOffset::new(7),
        ),
    );

    let mut builder = CstBuilder::new(SyntaxKind::Root);
    let checkpoint = builder.checkpoint();
    builder.token(name);
    builder.start_node_at(checkpoint, SyntaxKind::OptionalShape);
    builder.token(question);
    builder.finish_node();
    let root = builder.finish();

    let optional_nodes = root
        .children
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(node) if node.kind == SyntaxKind::OptionalShape => Some(node),
            CstElement::Node(_) | CstElement::Token(_) => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(optional_nodes.len(), 1);
    assert_eq!(optional_nodes[0].children.len(), 2);
    assert_eq!(optional_nodes[0].span.map(|span| span.end.get()), Some(7));
}

fn all_token_kinds(node: &tune_syntax::CstNode) -> Vec<TokenKind> {
    let mut kinds = Vec::new();
    collect_token_kinds(node, &mut kinds);
    kinds
}

fn collect_token_kinds(node: &tune_syntax::CstNode, kinds: &mut Vec<TokenKind>) {
    for child in &node.children {
        match child {
            CstElement::Node(node) => collect_token_kinds(node, kinds),
            CstElement::Token(token) => kinds.push(token.kind),
        }
    }
}
