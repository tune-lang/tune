use tune_syntax::{TokenKind, lex_with_file};

#[test]
fn lexes_core_keywords_and_punctuation() {
    let lexed = lex_with_file(tune_diagnostics::FileId(3), "pub let f(x): Int => x!");
    let kinds = significant_kinds(&lexed);

    assert_eq!(
        kinds,
        [
            TokenKind::KeywordPub,
            TokenKind::KeywordLet,
            TokenKind::Ident,
            TokenKind::LeftParen,
            TokenKind::Ident,
            TokenKind::RightParen,
            TokenKind::Colon,
            TokenKind::Ident,
            TokenKind::FatArrow,
            TokenKind::Ident,
            TokenKind::Bang,
            TokenKind::Eof,
        ]
    );
    assert!(lexed.diagnostics.is_empty());
}

#[test]
fn lexes_is_not_as_parser_level_operator_phrase() {
    let lexed = lex_with_file(tune_diagnostics::FileId(0), "name is not none");
    let kinds = significant_kinds(&lexed);

    assert_eq!(
        kinds,
        [
            TokenKind::Ident,
            TokenKind::KeywordIs,
            TokenKind::KeywordNot,
            TokenKind::KeywordNone,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn preserves_comments_and_whitespace_as_tokens() {
    let lexed = lex_with_file(tune_diagnostics::FileId(0), "-- docs\n-- comment\nlet");

    assert_eq!(lexed.tokens[0].kind, TokenKind::LineComment);
    assert_eq!(lexed.tokens[1].kind, TokenKind::Whitespace);
    assert_eq!(lexed.tokens[2].kind, TokenKind::LineComment);
    assert_eq!(lexed.tokens[3].kind, TokenKind::Whitespace);
    assert_eq!(lexed.tokens[4].kind, TokenKind::KeywordLet);
}

#[test]
fn treats_comments_as_trivia_without_losing_them() {
    let lexed = lex_with_file(tune_diagnostics::FileId(0), "-- docs\nlet value = 1");
    let kinds = significant_kinds(&lexed);

    assert_eq!(
        kinds,
        [
            TokenKind::KeywordLet,
            TokenKind::Ident,
            TokenKind::Equal,
            TokenKind::IntLiteral,
            TokenKind::Eof,
        ]
    );
    assert!(
        lexed
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::LineComment)
    );
}

#[test]
fn lexes_optional_shape_marker() {
    let lexed = lex_with_file(tune_diagnostics::FileId(0), "String?");
    let kinds = significant_kinds(&lexed);

    assert_eq!(
        kinds,
        [TokenKind::Ident, TokenKind::Question, TokenKind::Eof]
    );
}

#[test]
fn recognizes_string_and_multiline_string_literals() {
    let lexed = lex_with_file(
        tune_diagnostics::FileId(0),
        "\"hello\" \"\"\"multi\nline\"\"\"",
    );
    let kinds = significant_kinds(&lexed);

    assert_eq!(
        kinds,
        [
            TokenKind::StringLiteral,
            TokenKind::MultilineStringLiteral,
            TokenKind::Eof,
        ]
    );
    assert!(lexed.diagnostics.is_empty());
}

#[test]
fn records_byte_spans() {
    let lexed = lex_with_file(tune_diagnostics::FileId(9), "let name = 20");

    let first = &lexed.tokens[0];
    let int_literals = lexed
        .tokens
        .iter()
        .find(|token| token.kind == TokenKind::IntLiteral)
        .into_iter()
        .collect::<Vec<_>>();
    assert_eq!(int_literals.len(), 1);
    let last_value = int_literals[0];

    assert_eq!(first.span.file, tune_diagnostics::FileId(9));
    assert_eq!(first.span.start.get(), 0);
    assert_eq!(first.span.end.get(), 3);
    assert_eq!(last_value.span.start.get(), 11);
    assert_eq!(last_value.span.end.get(), 13);
}

#[test]
fn reports_unterminated_string() {
    let lexed = lex_with_file(tune_diagnostics::FileId(0), "\"unterminated");

    assert_eq!(lexed.tokens[0].kind, TokenKind::Error);
    assert_eq!(lexed.diagnostics.len(), 1);
    assert_eq!(lexed.diagnostics[0].title, "unterminated string literal");
}

fn significant_kinds(lexed: &tune_syntax::Lexed) -> Vec<TokenKind> {
    lexed
        .tokens
        .iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::LineComment))
        .map(|token| token.kind)
        .collect()
}
