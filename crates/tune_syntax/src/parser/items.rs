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
        if self.lookahead_significant(2) == Some(TokenKind::LeftParen) {
            self.parse_callable_decl();
        } else {
            self.parse_binding_decl();
        }
    }

    fn parse_callable_decl(&mut self) {
        self.start_node(SyntaxKind::CallableDecl);
        self.expect(TokenKind::KeywordLet, "expected `let`");
        self.skip_trivia();
        self.expect(TokenKind::Ident, "expected callable name");
        self.skip_trivia();
        self.parse_param_list();

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
                self.parse_expr_until_boundary();
                break;
            }

            self.update_depth_before_bump(&mut depth);
            self.bump();
        }

        self.finish_node();
    }

    fn parse_binding_decl(&mut self) {
        self.start_node(SyntaxKind::LetDecl);
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
                self.parse_expr_until_boundary();
                break;
            }

            self.update_depth_before_bump(&mut depth);
            self.bump();
        }

        self.finish_node();
    }

    fn parse_param_list(&mut self) {
        self.start_node(SyntaxKind::ParamList);
        self.expect(TokenKind::LeftParen, "expected `(`");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightParen) {
            self.parse_param();
            self.skip_trivia();
            if self.at(TokenKind::Comma) {
                self.bump();
                self.skip_trivia();
            } else if !self.at(TokenKind::RightParen) {
                self.error_at_current("expected `,` between parameters");
                break;
            }
        }

        self.expect(TokenKind::RightParen, "expected `)`");
        self.finish_node();
    }

    fn parse_param(&mut self) {
        self.start_node(SyntaxKind::Param);
        self.expect(TokenKind::Ident, "expected parameter name");
        self.skip_trivia();

        if self.at(TokenKind::Colon) {
            self.bump();
            self.skip_trivia();
            self.parse_shape();
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

    fn at_anonymous_callable_start(&self) -> bool {
        self.at(TokenKind::Ident)
            && self.current_text() == Some("_")
            && self.lookahead_significant(1) == Some(TokenKind::LeftParen)
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
