mod shape;

use crate::{CstBuilder, CstNode, SyntaxKind, Token, TokenKind, lex_with_file};
use tune_diagnostics::{Diagnostic, FileId, Span, codes};

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

    fn parse_top_level_item(&mut self) {
        match self.current_kind() {
            Some(TokenKind::KeywordPub) => self.parse_pub_decl(),
            Some(TokenKind::KeywordImport) => self.parse_simple_decl(SyntaxKind::ImportDecl),
            Some(TokenKind::KeywordTag) => self.parse_braced_decl(SyntaxKind::TagDecl),
            Some(TokenKind::KeywordStruct) => self.parse_braced_decl(SyntaxKind::StructDecl),
            Some(TokenKind::KeywordEnum) => self.parse_braced_decl(SyntaxKind::EnumDecl),
            Some(TokenKind::KeywordLet) => self.parse_let_decl(),
            Some(_) => self.parse_error_token("expected top-level declaration"),
            None => {}
        }
    }

    fn parse_pub_decl(&mut self) {
        self.start_node(SyntaxKind::PubDecl);
        self.expect(TokenKind::KeywordPub, "expected `pub`");
        self.skip_trivia();
        self.parse_top_level_item();
        self.finish_node();
    }

    fn parse_simple_decl(&mut self, kind: SyntaxKind) {
        self.start_node(kind);
        self.bump();
        self.consume_until_boundary();
        self.finish_node();
    }

    fn parse_braced_decl(&mut self, kind: SyntaxKind) {
        self.start_node(kind);
        self.bump();
        self.consume_until_block_end();
        self.finish_node();
    }

    fn parse_let_decl(&mut self) {
        let kind = if self.lookahead_significant(2) == Some(TokenKind::LeftParen) {
            SyntaxKind::CallableDecl
        } else {
            SyntaxKind::LetDecl
        };

        self.start_node(kind);
        self.expect(TokenKind::KeywordLet, "expected `let`");

        while !self.at(TokenKind::Eof) {
            if self.at_top_level_boundary() {
                break;
            }

            if self.at(TokenKind::Colon) {
                self.bump();
                self.skip_trivia();
                self.parse_shape();
                continue;
            }

            if self.at(TokenKind::Equal) {
                self.bump();
                self.skip_trivia();
                if self.at_anonymous_callable_start() {
                    self.parse_callable_value();
                    break;
                }
                continue;
            }

            self.bump();
        }

        self.finish_node();
    }

    fn parse_callable_value(&mut self) {
        self.start_node(SyntaxKind::CallableValue);
        self.consume_until_boundary();
        self.finish_node();
    }

    fn parse_error_token(&mut self, message: &'static str) {
        self.start_node(SyntaxKind::Error);
        self.error_at_current(message);
        self.bump();
        self.finish_node();
    }

    fn consume_until_boundary(&mut self) {
        let mut depth = 0u32;

        while !self.at(TokenKind::Eof) {
            match self.current_kind() {
                Some(TokenKind::LeftBrace | TokenKind::LeftParen | TokenKind::LeftBracket) => {
                    depth = depth.saturating_add(1);
                    self.bump();
                }
                Some(TokenKind::RightBrace | TokenKind::RightParen | TokenKind::RightBracket) => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    self.bump();
                }
                Some(TokenKind::KeywordLet | TokenKind::KeywordPub | TokenKind::KeywordImport)
                | Some(TokenKind::KeywordTag | TokenKind::KeywordStruct | TokenKind::KeywordEnum)
                    if depth == 0 =>
                {
                    break;
                }
                Some(_) => self.bump(),
                None => break,
            }
        }
    }

    fn consume_until_block_end(&mut self) {
        let mut depth = 0u32;
        let mut saw_block = false;

        while !self.at(TokenKind::Eof) {
            match self.current_kind() {
                Some(TokenKind::LeftBrace) => {
                    saw_block = true;
                    depth = depth.saturating_add(1);
                    self.bump();
                }
                Some(TokenKind::RightBrace) => {
                    self.bump();
                    if depth <= 1 && saw_block {
                        break;
                    }
                    depth = depth.saturating_sub(1);
                }
                Some(_) => self.bump(),
                None => break,
            }
        }

        if !saw_block {
            self.error_at_current("expected declaration body");
        }
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

    fn at_anonymous_callable_start(&self) -> bool {
        self.at(TokenKind::Ident)
            && self.current_text() == Some("_")
            && self.lookahead_significant(1) == Some(TokenKind::LeftParen)
    }

    fn at_top_level_boundary(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(TokenKind::KeywordLet | TokenKind::KeywordPub | TokenKind::KeywordImport)
                | Some(TokenKind::KeywordTag | TokenKind::KeywordStruct | TokenKind::KeywordEnum)
        )
    }

    pub(super) fn lookahead_significant(&self, n: usize) -> Option<TokenKind> {
        self.tokens
            .iter()
            .skip(self.cursor)
            .filter(|token| !crate::cst::is_trivia(token.kind))
            .nth(n)
            .map(|token| token.kind)
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
