use crate::token::{Token, TokenKind};
use tune_diagnostics::{ByteOffset, Diagnostic, FileId, Span, codes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lexed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn lex(source: &str) -> Vec<Token> {
    lex_with_file(FileId(0), source).tokens
}

#[must_use]
pub fn lex_with_file(file: FileId, source: &str) -> Lexed {
    let mut lexer = Lexer::new(file, source);
    lexer.lex_all();
    Lexed {
        tokens: lexer.tokens,
        diagnostics: lexer.diagnostics,
    }
}

struct Lexer<'src> {
    file: FileId,
    source: &'src str,
    offset: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl<'src> Lexer<'src> {
    fn new(file: FileId, source: &'src str) -> Self {
        Self {
            file,
            source,
            offset: 0,
            tokens: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn lex_all(&mut self) {
        while !self.is_at_end() {
            self.lex_one();
        }

        let eof = self.span(self.offset, self.offset);
        self.tokens.push(Token::new(TokenKind::Eof, eof));
    }

    fn lex_one(&mut self) {
        let start = self.offset;
        let Some(ch) = self.peek() else {
            return;
        };

        match ch {
            ch if ch.is_whitespace() => {
                self.lex_while(start, TokenKind::Whitespace, char::is_whitespace)
            }
            '-' if self.starts_with("--") => self.lex_line_comment(start, TokenKind::LineComment),
            '-' if self.starts_with("-/") => self.lex_block_comment(start),
            '"' if self.starts_with("\"\"\"") => self.lex_multiline_string(start),
            '"' => self.lex_string(start),
            '0'..='9' => self.lex_number(start),
            ch if is_ident_start(ch) => self.lex_ident_or_keyword(start),
            _ => self.lex_punctuation_or_error(start),
        }
    }

    fn lex_while(&mut self, start: usize, kind: TokenKind, predicate: fn(char) -> bool) {
        while self.peek().is_some_and(predicate) {
            self.bump();
        }

        self.push(kind, start, self.offset);
    }

    fn lex_line_comment(&mut self, start: usize, kind: TokenKind) {
        while self.peek().is_some_and(|ch| ch != '\n') {
            self.bump();
        }

        self.push(kind, start, self.offset);
    }

    fn lex_block_comment(&mut self, start: usize) {
        self.offset += 2;

        while !self.is_at_end() {
            if self.starts_with("/-") {
                self.offset += 2;
                self.push(TokenKind::BlockComment, start, self.offset);
                return;
            }

            self.bump();
        }

        self.push_error(start, self.offset, "unterminated block comment");
    }

    fn lex_string(&mut self, start: usize) {
        self.bump();

        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.bump();
                    self.push(TokenKind::StringLiteral, start, self.offset);
                    return;
                }
                '\\' => {
                    self.bump();
                    if !self.is_at_end() {
                        self.bump();
                    }
                }
                '\n' => break,
                _ => {
                    self.bump();
                }
            }
        }

        self.push_error(
            start,
            self.offset.max(start + 1),
            "unterminated string literal",
        );
    }

    fn lex_multiline_string(&mut self, start: usize) {
        self.offset += 3;

        while !self.is_at_end() {
            if self.starts_with("\"\"\"") {
                self.offset += 3;
                self.push(TokenKind::MultilineStringLiteral, start, self.offset);
                return;
            }

            self.bump();
        }

        self.push_error(start, self.offset, "unterminated multiline string literal");
    }

    fn lex_number(&mut self, start: usize) {
        if self.starts_with("0b") || self.starts_with("0B") {
            self.lex_radix_number(start, 2, |ch| matches!(ch, '0' | '1'));
            return;
        }
        if self.starts_with("0o") || self.starts_with("0O") {
            self.lex_radix_number(start, 2, |ch| matches!(ch, '0'..='7'));
            return;
        }
        if self.starts_with("0x") || self.starts_with("0X") {
            self.lex_radix_number(start, 2, |ch| ch.is_ascii_hexdigit());
            return;
        }

        while self
            .peek()
            .is_some_and(|ch| ch.is_ascii_digit() || ch == '_')
        {
            self.bump();
        }

        let mut kind = TokenKind::IntLiteral;

        if self.peek() == Some('.') && !self.starts_with("..") {
            let dot = self.offset;
            self.bump();

            if self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                kind = TokenKind::FloatLiteral;
                while self
                    .peek()
                    .is_some_and(|ch| ch.is_ascii_digit() || ch == '_')
                {
                    self.bump();
                }
            } else {
                self.offset = dot;
            }
        }

        self.push(kind, start, self.offset);
    }

    fn lex_radix_number(
        &mut self,
        start: usize,
        prefix_len: usize,
        valid_digit: impl Fn(char) -> bool,
    ) {
        self.offset += prefix_len;
        let digit_start = self.offset;
        while self.peek().is_some_and(|ch| valid_digit(ch) || ch == '_') {
            self.bump();
        }

        if self.offset == digit_start {
            self.push_error(start, self.offset, "expected digit after numeric prefix");
            return;
        }

        self.push(TokenKind::IntLiteral, start, self.offset);
    }

    fn lex_ident_or_keyword(&mut self, start: usize) {
        while self.peek().is_some_and(is_ident_continue) {
            self.bump();
        }

        let text = &self.source[start..self.offset];
        let kind = keyword_kind(text).unwrap_or(TokenKind::Ident);
        self.push(kind, start, self.offset);
    }

    fn lex_punctuation_or_error(&mut self, start: usize) {
        let kind = match self.peek() {
            Some('@') => single(TokenKind::At),
            Some('!') => single(TokenKind::Bang),
            Some('?') => single(TokenKind::Question),
            Some('~') if self.starts_with("~=") => double(TokenKind::TildeEqual),
            Some('~') => single(TokenKind::Tilde),
            Some('&') if self.starts_with("&=") => double(TokenKind::AmpEqual),
            Some('&') => single(TokenKind::Amp),
            Some('|') if self.starts_with("|=") => double(TokenKind::PipeEqual),
            Some('|') => single(TokenKind::Pipe),
            Some('^') if self.starts_with("^=") => double(TokenKind::CaretEqual),
            Some('^') => single(TokenKind::Caret),
            Some('+') if self.starts_with("+=") => double(TokenKind::PlusEqual),
            Some('+') => single(TokenKind::Plus),
            Some('-') if self.starts_with("->") => double(TokenKind::Arrow),
            Some('-') if self.starts_with("-=") => double(TokenKind::MinusEqual),
            Some('-') => single(TokenKind::Minus),
            Some('*') if self.starts_with("*=") => double(TokenKind::StarEqual),
            Some('*') => single(TokenKind::Star),
            Some('/') if self.starts_with("/=") => double(TokenKind::SlashEqual),
            Some('/') => single(TokenKind::Slash),
            Some('%') if self.starts_with("%=") => double(TokenKind::PercentEqual),
            Some('%') => single(TokenKind::Percent),
            Some('=') if self.starts_with("==") => double(TokenKind::EqualEqual),
            Some('=') if self.starts_with("=>") => double(TokenKind::FatArrow),
            Some('=') => single(TokenKind::Equal),
            Some('<') if self.starts_with("<<=") => TokenKind::ShiftLeftEqual,
            Some('<') if self.starts_with("<<") => double(TokenKind::ShiftLeft),
            Some('<') if self.starts_with("<=") => double(TokenKind::LessEqual),
            Some('<') => single(TokenKind::Less),
            Some('>') if self.starts_with(">>=") => TokenKind::ShiftRightEqual,
            Some('>') if self.starts_with(">>") => double(TokenKind::ShiftRight),
            Some('>') if self.starts_with(">=") => double(TokenKind::GreaterEqual),
            Some('>') => single(TokenKind::Greater),
            Some(':') if self.starts_with("::") => double(TokenKind::ColonColon),
            Some(':') => single(TokenKind::Colon),
            Some('.') if self.starts_with("..=") => TokenKind::DotDotEqual,
            Some('.') if self.starts_with("..") => double(TokenKind::DotDot),
            Some('.') => single(TokenKind::Dot),
            Some(',') => single(TokenKind::Comma),
            Some(';') => single(TokenKind::Semicolon),
            Some('(') => single(TokenKind::LeftParen),
            Some(')') => single(TokenKind::RightParen),
            Some('{') => single(TokenKind::LeftBrace),
            Some('}') => single(TokenKind::RightBrace),
            Some('[') => single(TokenKind::LeftBracket),
            Some(']') => single(TokenKind::RightBracket),
            _ => {
                self.bump();
                self.push_error(start, self.offset, "unrecognized character");
                return;
            }
        };

        self.offset += token_width(kind);
        self.push(kind, start, self.offset);
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token::new(kind, self.span(start, end)));
    }

    fn push_error(&mut self, start: usize, end: usize, message: &'static str) {
        let span = self.span(start, end);
        self.tokens.push(Token::new(TokenKind::Error, span));
        self.diagnostics
            .push(Diagnostic::error(codes::PARSE_ERROR, message, span, message).build());
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(
            self.file,
            ByteOffset::new(to_u32(start)),
            ByteOffset::new(to_u32(end)),
        )
    }

    fn starts_with(&self, needle: &str) -> bool {
        self.source[self.offset..].starts_with(needle)
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    fn is_at_end(&self) -> bool {
        self.offset >= self.source.len()
    }
}

fn keyword_kind(text: &str) -> Option<TokenKind> {
    Some(match text {
        "let" => TokenKind::KeywordLet,
        "struct" => TokenKind::KeywordStruct,
        "enum" => TokenKind::KeywordEnum,
        "tag" => TokenKind::KeywordTag,
        "if" => TokenKind::KeywordIf,
        "elif" => TokenKind::KeywordElif,
        "else" => TokenKind::KeywordElse,
        "match" => TokenKind::KeywordMatch,
        "for" => TokenKind::KeywordFor,
        "in" => TokenKind::KeywordIn,
        "while" => TokenKind::KeywordWhile,
        "loop" => TokenKind::KeywordLoop,
        "break" => TokenKind::KeywordBreak,
        "continue" => TokenKind::KeywordContinue,
        "return" => TokenKind::KeywordReturn,
        "spawn" => TokenKind::KeywordSpawn,
        "import" => TokenKind::KeywordImport,
        "pub" => TokenKind::KeywordPub,
        "panic" => TokenKind::KeywordPanic,
        "Never" => TokenKind::KeywordNever,
        "Ok" => TokenKind::KeywordOk,
        "Error" => TokenKind::KeywordError,
        "true" => TokenKind::KeywordTrue,
        "false" => TokenKind::KeywordFalse,
        "none" => TokenKind::KeywordNone,
        "self" => TokenKind::KeywordSelf,
        "not" => TokenKind::KeywordNot,
        "and" => TokenKind::KeywordAnd,
        "or" => TokenKind::KeywordOr,
        "is" => TokenKind::KeywordIs,
        _ => return None,
    })
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

const fn single(kind: TokenKind) -> TokenKind {
    kind
}

const fn double(kind: TokenKind) -> TokenKind {
    kind
}

const fn token_width(kind: TokenKind) -> usize {
    match kind {
        TokenKind::TildeEqual
        | TokenKind::PlusEqual
        | TokenKind::MinusEqual
        | TokenKind::StarEqual
        | TokenKind::SlashEqual
        | TokenKind::PercentEqual
        | TokenKind::AmpEqual
        | TokenKind::PipeEqual
        | TokenKind::CaretEqual
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::EqualEqual
        | TokenKind::LessEqual
        | TokenKind::GreaterEqual
        | TokenKind::ShiftLeft
        | TokenKind::ShiftRight
        | TokenKind::ColonColon
        | TokenKind::DotDot => 2,
        TokenKind::DotDotEqual | TokenKind::ShiftLeftEqual | TokenKind::ShiftRightEqual => 3,
        _ => 1,
    }
}

fn to_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}
