use crate::{SyntaxKind, TokenKind};

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_struct_expr(&mut self) {
        self.start_node(SyntaxKind::StructExpr);
        self.start_node(SyntaxKind::NameExpr);
        self.bump();
        self.finish_node();
        self.skip_inline_trivia();
        self.parse_struct_field_init_block();
        self.finish_node();
    }

    pub(super) fn at_struct_literal_start(&self) -> bool {
        self.at(TokenKind::Ident)
            && self
                .current_text()
                .and_then(|text| text.chars().next())
                .is_some_and(char::is_uppercase)
            && self.lookahead_significant(1) == Some(TokenKind::LeftBrace)
    }

    fn parse_struct_field_init_block(&mut self) {
        self.expect(TokenKind::LeftBrace, "expected `{`");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightBrace) {
            self.start_node(SyntaxKind::StructFieldInit);
            self.expect(TokenKind::Ident, "expected field name");
            self.skip_trivia();
            self.expect(TokenKind::Equal, "expected `=` in field initializer");
            self.skip_trivia();
            self.parse_expr();
            self.finish_node();

            self.skip_trivia();
            if self.at(TokenKind::Comma) || self.at(TokenKind::Semicolon) {
                self.bump();
                self.skip_trivia();
            } else if self.at(TokenKind::Whitespace) && self.current_text_has_newline() {
                self.skip_trivia();
            } else if !self.at(TokenKind::RightBrace) {
                self.error_at_current("expected `,`, `;`, or newline between field initializers");
                break;
            }
        }

        self.expect(TokenKind::RightBrace, "expected `}`");
    }
}
