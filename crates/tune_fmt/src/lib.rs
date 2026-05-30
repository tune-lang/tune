use tune_syntax::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent: String,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: "  ".to_owned(),
        }
    }
}

#[must_use]
pub fn format_source(source: &str) -> String {
    format_source_with_options(source, &FormatOptions::default())
}

#[must_use]
pub fn format_source_with_options(source: &str, options: &FormatOptions) -> String {
    Formatter::new(source, options).finish()
}

struct Formatter<'a> {
    source: &'a str,
    options: &'a FormatOptions,
    output: String,
    indent: usize,
    line_start: bool,
    previous: Option<TokenKind>,
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str, options: &'a FormatOptions) -> Self {
        Self {
            source,
            options,
            output: String::new(),
            indent: 0,
            line_start: true,
            previous: None,
        }
    }

    fn finish(mut self) -> String {
        for token in tune_syntax::lex(self.source) {
            if token.kind == TokenKind::Eof {
                break;
            }
            if token.kind == TokenKind::Whitespace {
                continue;
            }
            self.write_token(token);
        }
        self.trim_blank_lines();
        self.output.push('\n');
        self.output
    }

    fn write_token(&mut self, token: Token) {
        match token.kind {
            TokenKind::LineComment => self.write_line_comment(token),
            TokenKind::BlockComment => self.write_block_comment(token),
            TokenKind::LeftBrace => self.write_left_brace(),
            TokenKind::RightBrace => self.write_right_brace(),
            TokenKind::Semicolon => self.newline(),
            TokenKind::Comma => self.write_comma(),
            TokenKind::Colon => self.write_colon(),
            TokenKind::Dot | TokenKind::Question | TokenKind::Bang => self.write_attached(token),
            TokenKind::LeftParen | TokenKind::LeftBracket => self.write_open_delimiter(token),
            TokenKind::RightParen | TokenKind::RightBracket => self.write_close_delimiter(token),
            kind if is_binary_or_assignment(kind) => self.write_spaced(token),
            _ => self.write_plain(token),
        }
        self.previous = Some(token.kind);
    }

    fn write_line_comment(&mut self, token: Token) {
        if !self.line_start {
            self.output.push(' ');
        }
        self.write_indent_if_needed();
        self.output.push_str(self.text(token).trim_end());
        self.newline();
    }

    fn write_block_comment(&mut self, token: Token) {
        self.ensure_separator(token.kind);
        self.write_indent_if_needed();
        self.output.push_str(self.text(token).trim());
        self.newline();
    }

    fn write_left_brace(&mut self) {
        self.ensure_space_before_block();
        self.output.push('{');
        self.indent += 1;
        self.newline();
    }

    fn write_right_brace(&mut self) {
        if !self.line_start {
            self.newline();
        }
        self.indent = self.indent.saturating_sub(1);
        self.write_indent_if_needed();
        self.output.push('}');
        self.newline();
    }

    fn write_comma(&mut self) {
        self.trim_trailing_space();
        self.output.push(',');
        self.output.push(' ');
    }

    fn write_colon(&mut self) {
        self.trim_trailing_space();
        self.output.push(':');
        self.output.push(' ');
    }

    fn write_attached(&mut self, token: Token) {
        self.trim_trailing_space();
        self.output.push_str(self.text(token));
    }

    fn write_open_delimiter(&mut self, token: Token) {
        if token.kind == TokenKind::LeftBracket && starts_collection_literal(self.previous) {
            self.ensure_separator(token.kind);
        }
        self.output.push_str(self.text(token));
    }

    fn write_close_delimiter(&mut self, token: Token) {
        self.trim_trailing_space();
        self.output.push_str(self.text(token));
    }

    fn write_spaced(&mut self, token: Token) {
        self.ensure_space();
        self.output.push_str(self.text(token));
        self.output.push(' ');
    }

    fn write_plain(&mut self, token: Token) {
        self.ensure_separator(token.kind);
        self.write_indent_if_needed();
        self.output.push_str(self.text(token));
    }

    fn ensure_separator(&mut self, next: TokenKind) {
        if self.line_start || self.output.ends_with(' ') {
            return;
        }
        if let Some(previous) = self.previous
            && needs_space_between(previous, next)
        {
            self.output.push(' ');
        }
    }

    fn ensure_space_before_block(&mut self) {
        self.trim_trailing_space();
        if !self.line_start && !self.output.ends_with(' ') {
            self.output.push(' ');
        }
    }

    fn ensure_space(&mut self) {
        self.trim_trailing_space();
        if !self.line_start && !self.output.ends_with(' ') {
            self.output.push(' ');
        }
        self.write_indent_if_needed();
    }

    fn write_indent_if_needed(&mut self) {
        if self.line_start {
            for _ in 0..self.indent {
                self.output.push_str(&self.options.indent);
            }
            self.line_start = false;
        }
    }

    fn newline(&mut self) {
        self.trim_trailing_space();
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.line_start = true;
    }

    fn trim_blank_lines(&mut self) {
        while self.output.ends_with('\n') || self.output.ends_with(' ') {
            self.output.pop();
        }
    }

    fn trim_trailing_space(&mut self) {
        while self.output.ends_with(' ') {
            self.output.pop();
        }
    }

    fn text(&self, token: Token) -> &'a str {
        let start = token.span.start.get() as usize;
        let end = token.span.end.get() as usize;
        self.source.get(start..end).unwrap_or("")
    }
}

fn starts_collection_literal(previous: Option<TokenKind>) -> bool {
    matches!(
        previous,
        Some(TokenKind::Equal | TokenKind::FatArrow | TokenKind::KeywordReturn | TokenKind::Comma)
            | None
    )
}

fn needs_space_between(previous: TokenKind, next: TokenKind) -> bool {
    (is_word_like(previous) && is_word_like(next))
        || matches!(
            (previous, next),
            (TokenKind::KeywordPub, _)
                | (TokenKind::KeywordLet, _)
                | (TokenKind::KeywordStruct, _)
                | (TokenKind::KeywordEnum, _)
                | (TokenKind::KeywordTag, _)
                | (TokenKind::KeywordImport, _)
                | (TokenKind::KeywordReturn, _)
                | (TokenKind::KeywordSpawn, _)
                | (TokenKind::KeywordIf, _)
                | (TokenKind::KeywordElif, _)
                | (TokenKind::KeywordElse, _)
                | (TokenKind::KeywordMatch, _)
                | (TokenKind::KeywordFor, _)
                | (TokenKind::KeywordIn, _)
                | (TokenKind::KeywordWhile, _)
                | (TokenKind::KeywordLoop, _)
                | (_, TokenKind::KeywordIn)
        )
}

fn is_word_like(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident
            | TokenKind::IntLiteral
            | TokenKind::FloatLiteral
            | TokenKind::StringLiteral
            | TokenKind::MultilineStringLiteral
            | TokenKind::KeywordNever
            | TokenKind::KeywordOk
            | TokenKind::KeywordError
            | TokenKind::KeywordTrue
            | TokenKind::KeywordFalse
            | TokenKind::KeywordNone
            | TokenKind::KeywordSelf
    )
}

fn is_binary_or_assignment(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Equal
            | TokenKind::EqualEqual
            | TokenKind::TildeEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::Plus
            | TokenKind::PlusEqual
            | TokenKind::Minus
            | TokenKind::MinusEqual
            | TokenKind::Star
            | TokenKind::StarEqual
            | TokenKind::Slash
            | TokenKind::SlashEqual
            | TokenKind::Percent
            | TokenKind::PercentEqual
            | TokenKind::Amp
            | TokenKind::AmpEqual
            | TokenKind::Pipe
            | TokenKind::PipeEqual
            | TokenKind::Caret
            | TokenKind::CaretEqual
            | TokenKind::ShiftLeft
            | TokenKind::ShiftLeftEqual
            | TokenKind::ShiftRight
            | TokenKind::ShiftRightEqual
            | TokenKind::FatArrow
            | TokenKind::Arrow
            | TokenKind::DotDot
            | TokenKind::DotDotEqual
            | TokenKind::KeywordAnd
            | TokenKind::KeywordOr
            | TokenKind::KeywordIs
    )
}
