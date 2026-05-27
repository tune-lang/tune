mod expr;
mod items;
mod members;
mod shape;

use crate::{CstBuilder, CstNode, SyntaxKind, Token, TokenKind, lex_with_file};
use tune_diagnostics::{ByteOffset, Diagnostic, FileId, Span, codes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub cst: CstNode,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn parse(source: &str) -> Parsed {
    parse_with_file(FileId(0), source)
}

#[must_use]
pub fn parse_with_file(file: FileId, source: &str) -> Parsed {
    let lexed = lex_with_file(file, source);
    let mut parser = Parser::new(source, lexed.tokens, lexed.diagnostics);

    Parsed {
        cst: parser.parse_root(),
        diagnostics: parser.diagnostics,
    }
}

pub(super) struct Parser<'src> {
    pub(super) source: &'src str,
    pub(super) tokens: Vec<Token>,
    pub(super) cursor: usize,
    pub(super) builder: CstBuilder,
    pub(super) diagnostics: Vec<Diagnostic>,
}

impl<'src> Parser<'src> {
    fn new(source: &'src str, tokens: Vec<Token>, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            source,
            tokens,
            cursor: 0,
            builder: CstBuilder::new(SyntaxKind::Root),
            diagnostics,
        }
    }

    fn parse_root(&mut self) -> CstNode {
        while !self.at(TokenKind::Eof) {
            self.skip_trivia();
            if self.at(TokenKind::Eof) {
                break;
            }
            self.parse_top_level_item();
        }

        if self.at(TokenKind::Eof) {
            self.bump();
        }

        self.finish_tree()
    }

    pub(super) fn finish_tree(&mut self) -> CstNode {
        let replacement = CstBuilder::new(SyntaxKind::Root);
        let builder = core::mem::replace(&mut self.builder, replacement);
        builder.finish()
    }

    pub(super) fn at(&self, kind: TokenKind) -> bool {
        self.current().is_some_and(|token| token.kind == kind)
    }

    pub(super) fn current(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    pub(super) fn current_kind(&self) -> Option<TokenKind> {
        self.current().map(|token| token.kind)
    }

    pub(super) fn current_text(&self) -> Option<&str> {
        self.current().map(|token| self.token_text(token))
    }

    pub(super) fn token_text(&self, token: &Token) -> &'src str {
        let start = token.span.start.get() as usize;
        let end = token.span.end.get() as usize;
        &self.source[start..end]
    }

    pub(super) fn lookahead_significant(&self, n: usize) -> Option<TokenKind> {
        self.tokens
            .iter()
            .skip(self.cursor)
            .filter(|token| !crate::cst::is_trivia(token.kind))
            .nth(n)
            .map(|token| token.kind)
    }

    pub(super) fn at_top_level_boundary(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(TokenKind::KeywordLet | TokenKind::KeywordPub | TokenKind::KeywordImport)
                | Some(TokenKind::KeywordTag | TokenKind::KeywordStruct | TokenKind::KeywordEnum)
        )
    }

    pub(super) fn at_statement_boundary(&self) -> bool {
        self.at(TokenKind::Semicolon)
            || (self.at(TokenKind::Whitespace) && self.current_text_has_newline())
    }

    pub(super) fn current_text_has_newline(&self) -> bool {
        self.current_text()
            .is_some_and(|text| text.bytes().any(|byte| byte == b'\n'))
    }

    pub(super) fn bump(&mut self) {
        if let Some(token) = self.current().copied() {
            self.builder.token(token);
            self.cursor += 1;
        }
    }

    pub(super) fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind);
    }

    pub(super) fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    pub(super) fn skip_trivia(&mut self) {
        while self
            .current()
            .is_some_and(|token| crate::cst::is_trivia(token.kind))
        {
            self.bump();
        }
    }

    pub(super) fn skip_inline_trivia(&mut self) {
        while self
            .current()
            .is_some_and(|token| crate::cst::is_trivia(token.kind))
            && !self.current_text_has_newline()
        {
            self.bump();
        }
    }

    pub(super) fn skip_whitespace(&mut self) {
        while self.at(TokenKind::Whitespace) {
            self.bump();
        }
    }

    pub(super) fn at_shape_end(&self, end: TokenKind) -> bool {
        self.at(end) || (end == TokenKind::Greater && self.at(TokenKind::ShiftRight))
    }

    pub(super) fn expect_shape_end(&mut self, end: TokenKind, message: &'static str) -> bool {
        if self.at(end) {
            self.bump();
            true
        } else if end == TokenKind::Greater && self.at(TokenKind::ShiftRight) {
            self.bump_split_shift_right_as_greater();
            true
        } else {
            self.error_at_current(message);
            false
        }
    }

    fn bump_split_shift_right_as_greater(&mut self) {
        let Some(token) = self.current().copied() else {
            return;
        };
        debug_assert_eq!(token.kind, TokenKind::ShiftRight);

        let start = token.span.start.get();
        let middle = ByteOffset::new(start.saturating_add(1));
        let first = Token::new(
            TokenKind::Greater,
            Span::new(token.span.file, token.span.start, middle),
        );
        let second = Token::new(
            TokenKind::Greater,
            Span::new(token.span.file, middle, token.span.end),
        );

        self.builder.token(first);
        if let Some(current) = self.tokens.get_mut(self.cursor) {
            *current = second;
        }
    }

    pub(super) fn expect(&mut self, kind: TokenKind, message: &'static str) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            self.error_at_current(message);
            false
        }
    }

    pub(super) fn error_at_current(&mut self, message: &'static str) {
        let span = self
            .current()
            .map(|token| token.span)
            .or_else(|| self.tokens.last().map(|token| token.span));

        if let Some(span) = span {
            self.error(span, message);
        }
    }

    pub(super) fn error(&mut self, span: Span, message: &'static str) {
        self.diagnostics
            .push(Diagnostic::error(codes::PARSE_ERROR, message, span, message).build());
    }
}
