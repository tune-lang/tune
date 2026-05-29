use super::Parser;
use crate::{SyntaxKind, TokenKind};

impl<'src> Parser<'src> {
    pub(super) fn parse_shape(&mut self) {
        self.parse_union_shape();
    }

    fn parse_union_shape(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_postfix_shape();
        self.skip_whitespace();

        if !self.at(TokenKind::Pipe) {
            return;
        }

        self.builder
            .start_node_at(checkpoint, SyntaxKind::UnionShape);
        while self.at(TokenKind::Pipe) {
            self.bump();
            self.skip_whitespace();
            self.parse_postfix_shape();
            self.skip_whitespace();
        }
        self.finish_node();
    }

    fn parse_postfix_shape(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.parse_primary_shape();
        self.skip_whitespace();

        while self.at(TokenKind::Question) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::OptionalShape);
            self.bump();
            self.finish_node();
            self.skip_whitespace();
        }
    }

    fn parse_primary_shape(&mut self) {
        match self.current_kind() {
            Some(TokenKind::Ident | TokenKind::KeywordNever) => self.parse_named_or_generic_shape(),
            Some(TokenKind::LeftBracket) => self.parse_sequence_shape(),
            Some(TokenKind::LeftParen) => self.parse_parenthesized_or_callable_shape(),
            Some(TokenKind::LeftBrace) => self.parse_structural_shape(),
            Some(_) => {
                self.start_node(SyntaxKind::Error);
                self.error_at_current("expected shape");
                self.bump();
                self.finish_node();
            }
            None => self.error_at_current("expected shape"),
        }
    }

    fn parse_named_or_generic_shape(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.start_node(SyntaxKind::Shape);
        self.bump();
        self.finish_node();
        self.skip_whitespace();

        if self.at(TokenKind::Less) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::GenericShape);
            self.bump();
            self.skip_whitespace();
            if !self.at(TokenKind::Greater) {
                self.parse_shape_list(TokenKind::Greater);
            }
            self.expect_shape_end(TokenKind::Greater, "expected `>`");
            self.finish_node();
        }
    }

    fn parse_sequence_shape(&mut self) {
        self.start_node(SyntaxKind::SequenceShape);
        self.expect(TokenKind::LeftBracket, "expected `[`");
        self.skip_whitespace();
        if !self.at(TokenKind::RightBracket) {
            self.parse_shape();
        }
        self.skip_whitespace();
        self.expect(TokenKind::RightBracket, "expected `]`");
        self.finish_node();
    }

    fn parse_parenthesized_or_callable_shape(&mut self) {
        let checkpoint = self.builder.checkpoint();
        self.start_node(SyntaxKind::TupleShape);
        self.expect(TokenKind::LeftParen, "expected `(`");
        self.skip_whitespace();

        if !self.at(TokenKind::RightParen) {
            self.parse_shape_list(TokenKind::RightParen);
        }

        self.expect(TokenKind::RightParen, "expected `)`");
        self.finish_node();
        self.skip_whitespace();

        if self.at(TokenKind::Colon) {
            self.builder
                .start_node_at(checkpoint, SyntaxKind::CallableShape);
            self.bump();
            self.skip_whitespace();
            self.parse_shape();
            self.finish_node();
        }
    }

    fn parse_structural_shape(&mut self) {
        self.start_node(SyntaxKind::StructuralShape);
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

    pub(super) fn parse_shape_list(&mut self, end: TokenKind) {
        self.start_node(SyntaxKind::ShapeList);
        while !self.at(TokenKind::Eof) && !self.at_shape_end(end) {
            self.parse_shape();
            self.skip_whitespace();
            if self.at(TokenKind::Comma) {
                self.bump();
                self.skip_whitespace();
            } else if !self.at_shape_end(end) {
                self.error_at_current("expected `,` between shapes");
                break;
            }
        }
        self.finish_node();
    }
}
