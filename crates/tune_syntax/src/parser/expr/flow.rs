use crate::{SyntaxKind, TokenKind};

use super::super::Parser;

impl Parser<'_> {
    pub(super) fn parse_if_expr(&mut self) {
        self.start_node(SyntaxKind::IfExpr);
        self.expect(TokenKind::KeywordIf, "expected `if`");
        self.skip_trivia();
        self.parse_expr();
        self.skip_trivia();
        self.expect_conditional_body("expected `if` body");
        self.skip_trivia_before_if_continuation();

        while self.at(TokenKind::KeywordElif) {
            self.bump();
            self.skip_trivia();
            self.parse_expr();
            self.skip_trivia();
            self.expect_conditional_body("expected `elif` body");
            self.skip_trivia_before_if_continuation();
        }

        if self.at(TokenKind::KeywordElse) {
            self.bump();
            self.skip_trivia();
            self.expect_else_body("expected `else` body");
        }

        self.finish_node();
    }

    fn skip_trivia_before_if_continuation(&mut self) {
        if matches!(
            self.lookahead_significant(0),
            Some(TokenKind::KeywordElif | TokenKind::KeywordElse)
        ) {
            self.skip_trivia();
        }
    }

    pub(super) fn parse_match_expr(&mut self) {
        self.start_node(SyntaxKind::MatchExpr);
        self.expect(TokenKind::KeywordMatch, "expected `match`");
        self.skip_trivia();
        self.parse_expr();
        self.skip_trivia();
        self.expect(TokenKind::LeftBrace, "expected `match` body");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightBrace) {
            self.parse_match_arm();
            if !self.consume_expr_separator() {
                self.skip_trivia();
            }
        }

        self.expect(TokenKind::RightBrace, "expected `}`");
        self.finish_node();
    }

    pub(super) fn parse_while_expr(&mut self) {
        self.start_node(SyntaxKind::WhileExpr);
        self.expect(TokenKind::KeywordWhile, "expected `while`");
        self.skip_trivia();
        self.parse_expr();
        self.skip_trivia();
        self.expect_block_expr("expected `while` body");
        self.finish_node();
    }

    pub(super) fn parse_loop_expr(&mut self) {
        self.start_node(SyntaxKind::LoopExpr);
        self.expect(TokenKind::KeywordLoop, "expected `loop`");
        self.skip_trivia();
        self.expect_block_expr("expected `loop` body");
        self.finish_node();
    }

    pub(super) fn parse_break_expr(&mut self) {
        self.start_node(SyntaxKind::BreakExpr);
        self.expect(TokenKind::KeywordBreak, "expected `break`");
        self.finish_node();
    }

    pub(super) fn parse_continue_expr(&mut self) {
        self.start_node(SyntaxKind::ContinueExpr);
        self.expect(TokenKind::KeywordContinue, "expected `continue`");
        self.finish_node();
    }

    pub(super) fn parse_panic_expr(&mut self) {
        self.start_node(SyntaxKind::PanicExpr);
        self.expect(TokenKind::KeywordPanic, "expected `panic`");
        self.skip_trivia();
        if self.at(TokenKind::LeftParen) {
            self.parse_expr_list(TokenKind::RightParen);
        }
        self.finish_node();
    }

    fn parse_match_arm(&mut self) {
        self.start_node(SyntaxKind::MatchArm);
        let is_else = self.at(TokenKind::KeywordElse);
        self.parse_pattern();
        self.skip_trivia();
        if is_else {
            self.expect_else_body("expected `else` body");
        } else if self.at(TokenKind::LeftBrace) {
            self.parse_block_expr();
        } else {
            self.expect(TokenKind::FatArrow, "expected `=>`");
            self.skip_trivia();
            self.parse_expr();
        }
        self.finish_node();
    }

    fn expect_conditional_body(&mut self, message: &'static str) {
        if self.at(TokenKind::LeftBrace) {
            self.parse_block_expr();
        } else if self.at(TokenKind::FatArrow) {
            self.bump();
            self.skip_trivia();
            self.parse_expr();
        } else {
            self.error_at_current(message);
        }
    }

    fn expect_else_body(&mut self, message: &'static str) {
        if self.at(TokenKind::LeftBrace) {
            self.parse_block_expr();
        } else if !self.at(TokenKind::Eof) && !self.at(TokenKind::RightBrace) {
            self.parse_expr();
        } else {
            self.error_at_current(message);
        }
    }

    fn expect_block_expr(&mut self, message: &'static str) {
        if self.at(TokenKind::LeftBrace) {
            self.parse_block_expr();
        } else {
            self.error_at_current(message);
        }
    }
}
