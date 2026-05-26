use crate::cst::{CstNode, SyntaxKind};

pub fn parse(_source: &str) -> CstNode {
    CstNode {
        kind: SyntaxKind::Root,
        children: Vec::new(),
    }
}
