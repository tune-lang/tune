use crate::{SyntaxKind, TokenKind};

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_braced_decl(&mut self, kind: SyntaxKind) {
        self.start_node(kind);
        self.bump();
        self.skip_trivia();
        self.expect(TokenKind::Ident, "expected declaration name");
        self.skip_trivia();

        if self.at(TokenKind::LeftBrace) {
            self.parse_decl_body(kind);
        } else {
            self.error_at_current("expected declaration body");
        }

        self.finish_node();
    }

    fn parse_decl_body(&mut self, kind: SyntaxKind) {
        self.expect(TokenKind::LeftBrace, "expected `{`");
        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightBrace) {
            self.skip_trivia();
            if self.at(TokenKind::RightBrace) {
                break;
            }

            let before = self.cursor;
            match kind {
                SyntaxKind::StructDecl => self.parse_struct_member_decl(),
                SyntaxKind::TagDecl => self.parse_field_decl(),
                SyntaxKind::EnumDecl => self.parse_variant_decl(),
                _ => self.parse_member_error(),
            }
            self.consume_member_separator();
            if self.cursor == before {
                self.parse_member_error();
            }
        }
        self.expect(TokenKind::RightBrace, "expected `}`");
    }

    fn parse_struct_member_decl(&mut self) {
        match self.current_kind() {
            Some(TokenKind::LeftBracket) => self.parse_sequence_materializer_decl(),
            Some(TokenKind::Ident)
                if self.lookahead_significant(1) == Some(TokenKind::LeftParen) =>
            {
                self.parse_member_callable_decl();
            }
            Some(TokenKind::Ident)
                if self.lookahead_significant(1) == Some(TokenKind::LeftBracket) =>
            {
                self.parse_index_access_decl();
            }
            Some(TokenKind::Ident) => self.parse_field_decl(),
            Some(_) | None => self.parse_member_error(),
        }
    }

    fn parse_field_decl(&mut self) {
        self.start_node(SyntaxKind::FieldDecl);
        self.expect(TokenKind::Ident, "expected field name");
        self.skip_trivia();
        if self.at(TokenKind::Colon) {
            self.bump();
            self.skip_trivia();
            self.parse_shape();
        } else {
            self.error_at_current("expected field shape");
        }
        self.finish_node();
    }

    fn parse_member_callable_decl(&mut self) {
        self.start_node(SyntaxKind::MemberCallableDecl);
        self.expect(TokenKind::Ident, "expected callable member name");
        self.skip_trivia();
        self.parse_param_list();
        self.skip_trivia();

        if self.at(TokenKind::Colon) {
            self.bump();
            self.skip_trivia();
            self.parse_shape();
            self.skip_trivia();
        }

        if self.at(TokenKind::Equal) {
            self.bump();
            self.skip_trivia();
            self.parse_expr_until_boundary();
        }

        self.finish_node();
    }

    fn parse_sequence_materializer_decl(&mut self) {
        self.start_node(SyntaxKind::SequenceMaterializerDecl);
        self.expect(TokenKind::LeftBracket, "expected `[`");
        self.skip_trivia();
        if !self.at(TokenKind::RightBracket) {
            self.expect(TokenKind::Ident, "expected materializer parameter");
        }
        self.skip_trivia();
        self.expect(TokenKind::RightBracket, "expected `]`");
        self.skip_trivia();

        if self.at(TokenKind::Equal) {
            self.bump();
            self.skip_trivia();
            self.parse_expr_until_boundary();
        }

        self.finish_node();
    }

    fn parse_index_access_decl(&mut self) {
        self.start_node(SyntaxKind::IndexAccessDecl);
        self.expect(TokenKind::Ident, "expected indexed receiver name");
        self.skip_trivia();
        self.expect(TokenKind::LeftBracket, "expected `[`");
        self.skip_trivia();
        self.expect(TokenKind::Ident, "expected index parameter name");
        self.skip_trivia();

        if self.at(TokenKind::Colon) {
            self.bump();
            self.skip_trivia();
            self.parse_shape();
            self.skip_trivia();
        }

        self.expect(TokenKind::RightBracket, "expected `]`");
        self.skip_trivia();

        if self.at(TokenKind::Colon) {
            self.bump();
            self.skip_trivia();
            self.parse_shape();
            self.skip_trivia();
        }

        if self.at(TokenKind::Equal) {
            self.bump();
            self.skip_trivia();
            self.parse_expr_until_boundary();
        }

        self.finish_node();
    }

    fn parse_variant_decl(&mut self) {
        self.start_node(SyntaxKind::VariantDecl);
        self.expect_variant_name();
        self.skip_trivia();

        if self.at(TokenKind::LeftParen) {
            self.bump();
            self.skip_trivia();
            if !self.at(TokenKind::RightParen) {
                self.parse_shape_list(TokenKind::RightParen);
            }
            self.expect(TokenKind::RightParen, "expected `)`");
        }

        self.finish_node();
    }

    fn expect_variant_name(&mut self) {
        if matches!(
            self.current_kind(),
            Some(TokenKind::Ident | TokenKind::KeywordOk | TokenKind::KeywordError)
        ) {
            self.bump();
        } else {
            self.error_at_current("expected variant name");
        }
    }

    fn parse_member_error(&mut self) {
        self.start_node(SyntaxKind::Error);
        self.error_at_current("expected declaration member");
        self.bump();
        self.finish_node();
    }

    fn consume_member_separator(&mut self) {
        self.skip_trivia();
        if self.at(TokenKind::Comma) || self.at(TokenKind::Semicolon) {
            self.bump();
        }
    }
}
