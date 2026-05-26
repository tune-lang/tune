use crate::token::{Token, TokenKind};

pub fn lex(source: &str) -> Vec<Token> {
    // Skeleton: preserve trivia and recognize triple-quoted strings.
    vec![Token {
        kind: TokenKind::Eof,
        text: String::new(),
        start: source.len() as u32,
        end: source.len() as u32,
    }]
}
