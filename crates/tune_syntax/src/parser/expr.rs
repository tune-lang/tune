mod flow;
mod operators;
mod structs;

use crate::{SyntaxKind, TokenKind};

use super::Parser;

impl Parser<'_> {
    pub(super) fn parse_expr_until_boundary(&mut self) {
        self.parse_expr();
        self.skip_trivia();

        if self.at(TokenKind::Semicolon) {
            self.bump();
        }
    }

    pub(super) fn parse_expr(&mut self) {
        match self.current_kind() {
            Some(TokenKind::KeywordLet) => self.parse_let_expr(),
            Some(TokenKind::KeywordIf) => self.parse_if_expr(),
            Some(TokenKind::KeywordMatch) => self.parse_match_expr(),
            Some(TokenKind::KeywordReturn) => self.parse_return_expr(),
            Some(TokenKind::KeywordSpawn) => self.parse_spawn_expr(),
            Some(TokenKind::KeywordFor) => self.parse_for_expr(),
            Some(TokenKind::KeywordWhile) => self.parse_while_expr(),
            Some(TokenKind::KeywordLoop) => self.parse_loop_expr(),
            Some(TokenKind::KeywordBreak) => self.parse_break_expr(),
            Some(TokenKind::KeywordContinue) => self.parse_continue_expr(),
            Some(TokenKind::KeywordPanic) => self.parse_panic_expr(),
            Some(TokenKind::LeftBrace) => self.parse_block_expr(),
            Some(_) => self.parse_assignment_expr(),
            None => self.error_at_current("expected expression"),
        }
    }

    fn parse_assignment_expr(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_binary_expr(0);
        self.skip_inline_trivia();

        if self.at_assignment_operator() {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::AssignExpr);
            self.bump();
            self.skip_inline_trivia();
            self.parse_expr();
            self.finish_node();
        }
    }

    fn at_assignment_operator(&self) -> bool {
        matches!(
            self.current_kind(),
            Some(
                TokenKind::Equal
                    | TokenKind::PlusEqual
                    | TokenKind::MinusEqual
                    | TokenKind::StarEqual
                    | TokenKind::SlashEqual
                    | TokenKind::PercentEqual
                    | TokenKind::AmpEqual
                    | TokenKind::PipeEqual
                    | TokenKind::CaretEqual
                    | TokenKind::ShiftLeftEqual
                    | TokenKind::ShiftRightEqual
            )
        )
    }

    fn parse_let_expr(&mut self) {
        self.start_node(SyntaxKind::LetExpr);
        self.expect(TokenKind::KeywordLet, "expected `let`");
        self.skip_trivia();
        self.expect(TokenKind::Ident, "expected binding name");
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
            self.parse_expr();
        }

        self.finish_node();
    }

    fn parse_return_expr(&mut self) {
        self.start_node(SyntaxKind::ReturnExpr);
        self.expect(TokenKind::KeywordReturn, "expected `return`");
        if self.at_statement_boundary() || self.at(TokenKind::RightBrace) || self.at(TokenKind::Eof)
        {
            self.finish_node();
            return;
        }

        self.skip_trivia();

        if !self.at(TokenKind::Eof)
            && !self.at(TokenKind::Semicolon)
            && !self.at(TokenKind::RightBrace)
        {
            self.parse_expr();
        }

        self.finish_node();
    }

    fn parse_spawn_expr(&mut self) {
        self.start_node(SyntaxKind::SpawnExpr);
        self.expect(TokenKind::KeywordSpawn, "expected `spawn`");
        self.skip_trivia();
        self.parse_expr();
        self.finish_node();
    }

    fn parse_for_expr(&mut self) {
        self.start_node(SyntaxKind::ForExpr);
        self.expect(TokenKind::KeywordFor, "expected `for`");
        self.skip_trivia();
        self.parse_pattern();
        self.skip_trivia();
        self.expect(TokenKind::KeywordIn, "expected `in`");
        self.skip_trivia();
        self.parse_expr();
        self.skip_trivia();

        if self.at(TokenKind::LeftBrace) {
            self.parse_block_expr();
        } else {
            self.error_at_current("expected `for` body");
        }

        self.finish_node();
    }

    pub(super) fn parse_pattern(&mut self) {
        self.start_node(SyntaxKind::Pattern);
        match self.current_kind() {
            Some(
                TokenKind::Ident
                | TokenKind::KeywordSelf
                | TokenKind::KeywordOk
                | TokenKind::KeywordError
                | TokenKind::KeywordElse,
            ) => {
                self.bump();
                self.skip_trivia();
                if self.at(TokenKind::LeftParen) {
                    self.parse_pattern_list();
                }
            }
            Some(TokenKind::LeftBrace) => self.parse_structural_pattern(),
            Some(TokenKind::LeftParen) => self.parse_pattern_list(),
            Some(_) | None => self.error_at_current("expected pattern"),
        }
        self.finish_node();
    }

    fn parse_structural_pattern(&mut self) {
        self.start_node(SyntaxKind::StructuralPattern);
        self.expect(TokenKind::LeftBrace, "expected `{`");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightBrace) {
            self.parse_structural_requirement();
            self.skip_trivia();
            if self.at(TokenKind::Comma) || self.at(TokenKind::Semicolon) {
                self.bump();
                self.skip_trivia();
            } else if !self.at(TokenKind::RightBrace) {
                self.error_at_current("expected `,` between structural requirements");
                break;
            }
        }

        self.expect(TokenKind::RightBrace, "expected `}`");
        self.finish_node();
    }

    fn parse_structural_requirement(&mut self) {
        self.start_node(SyntaxKind::StructuralRequirement);
        self.expect(TokenKind::Ident, "expected structural requirement name");
        self.skip_whitespace();

        if self.at(TokenKind::LeftParen) {
            self.bump();
            self.skip_whitespace();
            if !self.at(TokenKind::RightParen) {
                self.parse_shape_list(TokenKind::RightParen);
            }
            self.expect(TokenKind::RightParen, "expected `)`");
            self.skip_whitespace();
        }

        if self.at(TokenKind::Colon) {
            self.bump();
            self.skip_whitespace();
            self.parse_shape();
        }
        self.finish_node();
    }

    fn parse_pattern_list(&mut self) {
        self.start_node(SyntaxKind::PatternList);
        self.expect(TokenKind::LeftParen, "expected `(`");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightParen) {
            self.parse_pattern();
            self.skip_trivia();
            if self.at(TokenKind::Comma) {
                self.bump();
                self.skip_trivia();
            } else if !self.at(TokenKind::RightParen) {
                self.error_at_current("expected `,` between patterns");
                break;
            }
        }

        self.expect(TokenKind::RightParen, "expected `)`");
        self.finish_node();
    }

    pub(super) fn parse_block_expr(&mut self) {
        self.start_node(SyntaxKind::Block);
        self.expect(TokenKind::LeftBrace, "expected `{`");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightBrace) {
            self.parse_expr();
            if self.consume_expr_separator() {
                continue;
            }

            self.skip_trivia();
            if !self.at(TokenKind::RightBrace) {
                self.error_at_current("expected `;` or newline between expressions");
            }
        }

        self.expect(TokenKind::RightBrace, "expected `}`");
        self.finish_node();
    }

    fn parse_postfix_expr(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_primary_expr();
        self.skip_inline_trivia();

        loop {
            match self.current_kind() {
                Some(TokenKind::LeftParen) => {
                    self.builder.start_node_at(checkpoint, SyntaxKind::CallExpr);
                    self.parse_expr_list(TokenKind::RightParen);
                    self.finish_node();
                }
                Some(TokenKind::Dot) => {
                    self.builder
                        .start_node_at(checkpoint, SyntaxKind::FieldExpr);
                    self.bump();
                    self.skip_trivia();
                    self.expect(TokenKind::Ident, "expected field name");
                    self.finish_node();
                }
                Some(TokenKind::LeftBracket) => {
                    self.builder
                        .start_node_at(checkpoint, SyntaxKind::IndexExpr);
                    self.bump();
                    self.skip_trivia();
                    if !self.at(TokenKind::RightBracket) {
                        self.parse_expr();
                    }
                    self.expect(TokenKind::RightBracket, "expected `]`");
                    self.finish_node();
                }
                Some(TokenKind::Bang) => {
                    self.builder
                        .start_node_at(checkpoint, SyntaxKind::PropagateExpr);
                    self.bump();
                    self.finish_node();
                }
                _ => break,
            }

            self.skip_inline_trivia();
        }
    }

    fn parse_expr_list(&mut self, end: TokenKind) {
        self.expect(TokenKind::LeftParen, "expected `(`");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(end) {
            self.parse_expr();
            self.skip_trivia();
            if self.at(TokenKind::Comma) {
                self.bump();
                self.skip_trivia();
            } else if !self.at(end) {
                self.error_at_current("expected `,` between expressions");
                break;
            }
        }

        self.expect(end, "expected expression list closer");
    }

    pub(super) fn consume_expr_separator(&mut self) -> bool {
        if self.at(TokenKind::Semicolon) {
            self.bump();
            self.skip_trivia();
            return true;
        }

        if self.at(TokenKind::Whitespace) && self.current_text_has_newline() {
            self.skip_trivia();
            return true;
        }

        false
    }

    fn parse_primary_expr(&mut self) {
        match self.current_kind() {
            Some(
                TokenKind::IntLiteral
                | TokenKind::FloatLiteral
                | TokenKind::StringLiteral
                | TokenKind::MultilineStringLiteral
                | TokenKind::KeywordTrue
                | TokenKind::KeywordFalse
                | TokenKind::KeywordNone,
            ) => {
                self.start_node(SyntaxKind::LiteralExpr);
                self.bump();
                self.finish_node();
            }
            Some(
                TokenKind::Ident
                | TokenKind::KeywordSelf
                | TokenKind::KeywordOk
                | TokenKind::KeywordError,
            ) => {
                if self.at_anonymous_callable_start() {
                    self.parse_callable_value();
                } else if self.at_struct_literal_start() {
                    self.parse_struct_expr();
                } else {
                    self.start_node(SyntaxKind::NameExpr);
                    self.bump();
                    self.finish_node();
                }
            }
            Some(TokenKind::LeftParen) => {
                self.start_node(SyntaxKind::TupleExpr);
                self.bump();
                self.skip_trivia();

                if self.at(TokenKind::RightParen) {
                    self.bump();
                    self.finish_node();
                    return;
                }

                self.parse_expr();
                self.skip_trivia();

                if self.at(TokenKind::Comma) {
                    while self.at(TokenKind::Comma) {
                        self.bump();
                        self.skip_trivia();
                        if self.at(TokenKind::RightParen) {
                            break;
                        }
                        self.parse_expr();
                        self.skip_trivia();
                    }
                } else if !self.at(TokenKind::RightParen) {
                    self.error_at_current("expected `,` or `)` after parenthesized expression");
                    self.parse_expr();
                }

                self.expect(TokenKind::RightParen, "expected `)`");
                self.finish_node();
            }
            Some(TokenKind::LeftBracket) => self.parse_sequence_expr(),
            Some(_) => {
                self.start_node(SyntaxKind::Error);
                self.error_at_current("expected expression");
                self.bump();
                self.finish_node();
            }
            None => self.error_at_current("expected expression"),
        }
    }

    fn parse_sequence_expr(&mut self) {
        self.start_node(SyntaxKind::SequenceExpr);
        self.expect(TokenKind::LeftBracket, "expected `[`");
        self.skip_trivia();

        while !self.at(TokenKind::Eof) && !self.at(TokenKind::RightBracket) {
            self.parse_expr();
            self.skip_trivia();
            if self.at(TokenKind::Comma) {
                self.bump();
                self.skip_trivia();
            } else if !self.at(TokenKind::RightBracket) {
                self.error_at_current("expected `,` between expressions");
                break;
            }
        }

        self.expect(TokenKind::RightBracket, "expected `]`");
        self.finish_node();
    }

    fn parse_callable_value(&mut self) {
        self.start_node(SyntaxKind::CallableValue);
        self.expect(TokenKind::Ident, "expected `_`");
        self.skip_trivia();
        self.parse_param_list();
        self.skip_trivia();

        if self.at(TokenKind::Colon) {
            self.bump();
            self.skip_trivia();
            self.parse_shape();
            self.skip_trivia();
        }

        self.expect(TokenKind::Equal, "expected `=`");
        self.skip_trivia();
        self.parse_expr();
        self.finish_node();
    }

    fn at_anonymous_callable_start(&self) -> bool {
        self.at(TokenKind::Ident)
            && self.current_text() == Some("_")
            && self.lookahead_significant(1) == Some(TokenKind::LeftParen)
    }
}
