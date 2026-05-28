use crate::{SyntaxKind, TokenKind};

use super::super::Parser;

impl Parser<'_> {
    pub(super) fn parse_binary_expr(&mut self, min_precedence: u8) {
        let checkpoint = self.builder.checkpoint();
        self.parse_unary_expr();
        self.skip_inline_trivia();

        while let Some(precedence) = self.current_binary_precedence() {
            if precedence < min_precedence {
                break;
            }

            self.builder
                .start_node_at(checkpoint, SyntaxKind::BinaryExpr);
            self.consume_binary_operator();
            self.skip_inline_trivia();
            self.parse_binary_expr(precedence.saturating_add(1));
            self.finish_node();
            self.skip_inline_trivia();
        }
    }

    fn parse_unary_expr(&mut self) {
        if matches!(
            self.current_kind(),
            Some(TokenKind::KeywordNot | TokenKind::Minus | TokenKind::Tilde)
        ) {
            self.start_node(SyntaxKind::UnaryExpr);
            self.bump();
            self.skip_inline_trivia();
            self.parse_unary_expr();
            self.finish_node();
        } else {
            self.parse_postfix_expr();
        }
    }

    fn current_binary_precedence(&self) -> Option<u8> {
        let precedence = match self.current_kind()? {
            TokenKind::DotDot | TokenKind::DotDotEqual => 2,
            TokenKind::KeywordIs
            | TokenKind::EqualEqual
            | TokenKind::TildeEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => 3,
            TokenKind::KeywordOr | TokenKind::Pipe => 4,
            TokenKind::Caret => 5,
            TokenKind::KeywordAnd | TokenKind::Amp => 6,
            TokenKind::ShiftLeft | TokenKind::ShiftRight => 7,
            TokenKind::Plus | TokenKind::Minus => 8,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => 9,
            _ => return None,
        };

        Some(precedence)
    }

    fn consume_binary_operator(&mut self) {
        if self.at(TokenKind::KeywordIs) {
            self.bump();
            self.skip_inline_trivia();
            if self.at(TokenKind::KeywordNot) {
                self.bump();
            }
        } else {
            self.bump();
        }
    }
}
