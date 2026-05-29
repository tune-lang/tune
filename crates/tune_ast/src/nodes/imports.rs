use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::text::first_string_text;

#[derive(Debug, Clone, Copy)]
pub struct ImportDecl<'tree> {
    node: &'tree CstNode,
}

impl<'tree> AstNode<'tree> for ImportDecl<'tree> {
    const KIND: SyntaxKind = SyntaxKind::ImportDecl;

    fn cast(node: &'tree CstNode) -> Option<Self> {
        (node.kind == Self::KIND).then_some(Self { node })
    }

    fn syntax(&self) -> &'tree CstNode {
        self.node
    }
}

impl<'tree> ImportDecl<'tree> {
    #[must_use]
    pub fn path(self, source: &str) -> Option<&str> {
        first_string_text(self.node, source)
            .and_then(|text| text.strip_prefix('"'))
            .and_then(|text| text.strip_suffix('"'))
    }

    #[must_use]
    pub fn selector(self, source: &str) -> ImportSelector {
        let mut past_path = false;
        let mut past_dot = false;
        let mut in_group = false;
        let mut names = Vec::new();

        for child in &self.node.children {
            let CstElement::Token(token) = child else {
                continue;
            };
            match token.kind {
                TokenKind::StringLiteral => past_path = true,
                TokenKind::Dot if past_path => past_dot = true,
                TokenKind::LeftBrace if past_dot => in_group = true,
                TokenKind::RightBrace if in_group => break,
                TokenKind::Ident if past_dot => {
                    let start = token.span.start.get() as usize;
                    let end = token.span.end.get() as usize;
                    if let Some(name) = source.get(start..end) {
                        names.push(name.to_owned());
                    }
                    if !in_group {
                        break;
                    }
                }
                _ => {}
            }
        }

        match names.as_slice() {
            [] => ImportSelector::Module,
            [name] if !in_group => ImportSelector::Member(name.clone()),
            _ => ImportSelector::Members(names),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSelector {
    Module,
    Member(String),
    Members(Vec<String>),
}
