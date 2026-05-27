use crate::{SyntaxKind, TokenKind};

use super::super::Parser;

impl Parser<'_> {
    pub(super) fn parse_if_expr(&mut self) {
        self.start_node(SyntaxKind::IfExpr);
        self.expect(TokenKind::KeywordIf, "expected `if`");
        self.skip_trivia();
        self.parse_expr();
        self.skip_trivia();
        self.expect_block_expr("expected `if` body");
        self.skip_trivia();

        while self.at(TokenKind::KeywordElif) {
            self.bump();
            self.skip_trivia();
            self.parse_expr();
            self.skip_trivia();
            self.expect_block_expr("expected `elif` body");
            self.skip_trivia();
        }

        if self.at(TokenKind::KeywordElse) {
            self.bump();
            self.skip_trivia();
            self.expect_block_expr("expected `else` body");
        }

        self.finish_node();
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
        self.parse_pattern();
        self.skip_trivia();
        self.expect(TokenKind::FatArrow, "expected `=>`");
        self.skip_trivia();
        self.parse_expr();
        self.finish_node();
    }

    fn expect_block_expr(&mut self, message: &'static str) {
        if self.at(TokenKind::LeftBrace) {
            self.parse_block_expr();
        } else {
            self.error_at_current(message);
        }
    }
}
