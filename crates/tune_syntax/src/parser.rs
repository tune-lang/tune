use crate::{CstBuilder, CstNode, SyntaxKind, lex};

#[must_use]
pub fn parse(_source: &str) -> CstNode {
    let mut builder = CstBuilder::new(SyntaxKind::Root);

    for token in lex(_source) {
        builder.token(token);
    }

    builder.finish()
}
