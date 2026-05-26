use tune_ast::AstNode;
use tune_ast::nodes::{Expr as AstExpr, LiteralExpr};
use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::ExprId;
use crate::expr::{Expr, ExprKind, LiteralKind};
use crate::pattern::{Pattern, PatternKind};

#[derive(Default)]
pub(super) struct ExprLowerer {
    next_id: u32,
}

impl ExprLowerer {
    pub(super) fn lower(&mut self, source: &str, expr: AstExpr<'_>) -> Expr {
        let id = self.alloc_id();
        let span = expr.syntax().span;
        let kind = match expr {
            AstExpr::Missing(_) => ExprKind::Missing,
            AstExpr::Literal(node) => {
                literal_kind(source, node).map_or(ExprKind::Missing, ExprKind::Literal)
            }
            AstExpr::Name(node) => node
                .name(source)
                .map(|name| ExprKind::Name(name.to_owned()))
                .unwrap_or(ExprKind::Missing),
            AstExpr::Call(_) => self.lower_call(source, expr),
            AstExpr::Field(node) => self.lower_field(source, expr, node.field_name(source)),
            AstExpr::Index(_) => self.lower_index(source, expr),
            AstExpr::Propagate(_) => self.lower_unary(source, expr, ExprKind::Propagate),
            AstExpr::Spawn(_) => self.lower_unary(source, expr, ExprKind::Spawn),
            AstExpr::For(_) => self.lower_for(source, expr),
            AstExpr::Block(node) => ExprKind::Block(
                node.exprs()
                    .into_iter()
                    .map(|child| self.lower(source, child))
                    .collect(),
            ),
        };

        Expr { id, span, kind }
    }

    fn lower_call(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let Some(callee) = children.next() else {
            return ExprKind::Missing;
        };

        ExprKind::Call {
            callee: Box::new(self.lower(source, callee)),
            args: children.map(|arg| self.lower(source, arg)).collect(),
        }
    }

    fn lower_field(&mut self, source: &str, expr: AstExpr<'_>, name: Option<&str>) -> ExprKind {
        let Some(base) = expr.child_exprs().into_iter().next() else {
            return ExprKind::Missing;
        };

        ExprKind::Field {
            base: Box::new(self.lower(source, base)),
            name: name.map(str::to_owned),
        }
    }

    fn lower_index(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let (Some(base), Some(index)) = (children.next(), children.next()) else {
            return ExprKind::Missing;
        };

        ExprKind::Index {
            base: Box::new(self.lower(source, base)),
            index: Box::new(self.lower(source, index)),
        }
    }

    fn lower_unary(
        &mut self,
        source: &str,
        expr: AstExpr<'_>,
        wrap: impl FnOnce(Box<Expr>) -> ExprKind,
    ) -> ExprKind {
        let Some(inner) = expr.child_exprs().into_iter().next() else {
            return ExprKind::Missing;
        };

        wrap(Box::new(self.lower(source, inner)))
    }

    fn lower_for(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let (Some(iterable), Some(body)) = (children.next(), children.next()) else {
            return ExprKind::Missing;
        };

        ExprKind::For {
            pattern: lower_pattern(source, expr.syntax()),
            iterable: Box::new(self.lower(source, iterable)),
            body: Box::new(self.lower(source, body)),
        }
    }

    fn alloc_id(&mut self) -> ExprId {
        let id = ExprId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        id
    }
}

fn literal_kind(source: &str, node: LiteralExpr<'_>) -> Option<LiteralKind> {
    let text = node.text(source)?;
    match first_token_kind(node.syntax())? {
        TokenKind::IntLiteral => Some(LiteralKind::Int(text.to_owned())),
        TokenKind::FloatLiteral => Some(LiteralKind::Float(text.to_owned())),
        TokenKind::StringLiteral | TokenKind::MultilineStringLiteral => {
            Some(LiteralKind::String(text.to_owned()))
        }
        TokenKind::KeywordTrue => Some(LiteralKind::Bool(true)),
        TokenKind::KeywordFalse => Some(LiteralKind::Bool(false)),
        TokenKind::KeywordNone => Some(LiteralKind::None),
        _ => None,
    }
}

fn first_token_kind(node: &CstNode) -> Option<TokenKind> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) => Some(token.kind),
        CstElement::Node(_) => None,
    })
}

fn lower_pattern(source: &str, node: &CstNode) -> Pattern {
    let Some(pattern) = node.children.iter().find_map(|child| match child {
        CstElement::Node(node) if node.kind == SyntaxKind::Pattern => Some(node),
        CstElement::Node(_) | CstElement::Token(_) => None,
    }) else {
        return Pattern {
            span: node.span,
            kind: PatternKind::Hole,
        };
    };

    let name = pattern.children.iter().find_map(|child| match child {
        CstElement::Token(token)
            if matches!(token.kind, TokenKind::Ident | TokenKind::KeywordSelf) =>
        {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(_) | CstElement::Token(_) => None,
    });

    Pattern {
        span: pattern.span,
        kind: name.map_or(PatternKind::Hole, |name| {
            PatternKind::Binding(name.to_owned())
        }),
    }
}
