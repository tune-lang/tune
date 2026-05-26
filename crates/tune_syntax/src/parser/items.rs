use crate::{SyntaxKind, TokenKind};

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_top_level_item(&mut self) {
        match self.current_kind() {
            Some(TokenKind::At) => self.parse_tag_application(),
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

    fn parse_tag_application(&mut self) {
        self.start_node(SyntaxKind::TagApplication);
        self.expect(TokenKind::At, "expected `@`");
        self.skip_trivia();
        self.expect(TokenKind::Ident, "expected tag name");
        self.skip_trivia();

        if let Some(TokenKind::LeftParen | TokenKind::LeftBrace | TokenKind::LeftBracket) =
            self.current_kind()
        {
            self.consume_balanced_group();
        }

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

    fn consume_balanced_group(&mut self) {
        let mut depth = 0u32;

        while !self.at(TokenKind::Eof) {
            match self.current_kind() {
                Some(TokenKind::LeftBrace | TokenKind::LeftParen | TokenKind::LeftBracket) => {
                    depth = depth.saturating_add(1);
                    self.bump();
                }
                Some(TokenKind::RightBrace | TokenKind::RightParen | TokenKind::RightBracket) => {
                    self.bump();
                    if depth <= 1 {
                        break;
                    }
                    depth = depth.saturating_sub(1);
                }
                Some(_) => self.bump(),
                None => break,
            }
        }
    }

    fn parse_let_decl(&mut self) {
        let kind = if self.lookahead_significant(2) == Some(TokenKind::LeftParen) {
            SyntaxKind::CallableDecl
        } else {
            SyntaxKind::LetDecl
        };

        self.start_node(kind);
        self.expect(TokenKind::KeywordLet, "expected `let`");
        let mut depth = 0u32;

        while !self.at(TokenKind::Eof) {
            if depth == 0 && self.at_statement_boundary() {
                if self.at(TokenKind::Semicolon) {
                    self.bump();
                }
                break;
            }

            if depth == 0 && self.at_top_level_boundary() {
                break;
            }

            if depth == 0 && self.at(TokenKind::Colon) {
                self.bump();
                self.skip_trivia();
                self.parse_shape();
                continue;
            }

            if depth == 0 && self.at(TokenKind::Equal) {
                self.bump();
                self.skip_trivia();
                if self.at_anonymous_callable_start() {
                    self.parse_callable_value();
                    break;
                }
                continue;
            }

            self.update_depth_before_bump(&mut depth);
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
                Some(TokenKind::Semicolon) if depth == 0 => {
                    self.bump();
                    break;
                }
                Some(TokenKind::Whitespace) if depth == 0 && self.current_text_has_newline() => {
                    break;
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

    fn at_statement_boundary(&self) -> bool {
        self.at(TokenKind::Semicolon)
            || (self.at(TokenKind::Whitespace) && self.current_text_has_newline())
    }

    fn current_text_has_newline(&self) -> bool {
        self.current_text()
            .is_some_and(|text| text.bytes().any(|byte| byte == b'\n'))
    }

    fn update_depth_before_bump(&self, depth: &mut u32) {
        match self.current_kind() {
            Some(TokenKind::LeftBrace | TokenKind::LeftParen | TokenKind::LeftBracket) => {
                *depth = depth.saturating_add(1);
            }
            Some(TokenKind::RightBrace | TokenKind::RightParen | TokenKind::RightBracket) => {
                *depth = depth.saturating_sub(1);
            }
            Some(_) | None => {}
        }
    }
}
