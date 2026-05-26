use tune_syntax::{Token, TokenKind};

#[derive(Debug, Clone, Copy)]
pub struct Comment {
    token: Token,
}

impl Comment {
    #[must_use]
    pub fn cast(token: Token) -> Option<Self> {
        matches!(token.kind, TokenKind::LineComment | TokenKind::BlockComment)
            .then_some(Self { token })
    }

    #[must_use]
    pub const fn token(self) -> Token {
        self.token
    }

    #[must_use]
    pub fn source_text(self, source: &str) -> Option<&str> {
        let start = self.token.span.start.get() as usize;
        let end = self.token.span.end.get() as usize;
        source.get(start..end)
    }

    #[must_use]
    pub fn doc_text(self, source: &str) -> Option<String> {
        let text = self.source_text(source)?;
        match self.token.kind {
            TokenKind::LineComment => text.strip_prefix("--").map(clean_doc_line),
            TokenKind::BlockComment => text
                .strip_prefix("-/")
                .and_then(|text| text.strip_suffix("/-"))
                .map(clean_doc_block),
            _ => None,
        }
    }
}

fn clean_doc_line(text: &str) -> String {
    text.trim().to_owned()
}

fn clean_doc_block(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
