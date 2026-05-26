use crate::{CstBuilder, CstNode, SyntaxKind, lex_with_file};
use tune_diagnostics::{Diagnostic, FileId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub cst: CstNode,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn parse(source: &str) -> Parsed {
    parse_with_file(FileId(0), source)
}

#[must_use]
pub fn parse_with_file(file: FileId, source: &str) -> Parsed {
    let lexed = lex_with_file(file, source);
    let mut builder = CstBuilder::new(SyntaxKind::Root);

    for token in lexed.tokens {
        builder.token(token);
    }

    Parsed {
        cst: builder.finish(),
        diagnostics: lexed.diagnostics,
    }
}
