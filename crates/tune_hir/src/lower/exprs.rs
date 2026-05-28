use tune_ast::AstNode;
use tune_ast::nodes::{CallableParam, Expr as AstExpr, LiteralExpr, ParamList, Shape as AstShape};
use tune_syntax::{CstElement, CstNode, TokenKind};

use crate::ExprId;
mod flow;
mod patterns;
mod structs;

use super::shapes::lower_shape;
use crate::expr::{BinaryOp, Expr, ExprKind, ExprParam, LiteralKind, UnaryOp};
use patterns::lower_pattern;

#[derive(Default)]
pub(super) struct ExprLowerer {
    next_id: u64,
}

impl ExprLowerer {
    pub(super) fn lower(&mut self, source: &str, expr: AstExpr<'_>) -> Expr {
        let id = self.alloc_id();
        let span = expr.syntax().span;
        let kind = match expr {
            AstExpr::Missing(_) => ExprKind::Missing,
            AstExpr::Group(_) => self.lower_group(source, expr),
            AstExpr::Literal(node) => {
                literal_kind(source, node).map_or(ExprKind::Missing, ExprKind::Literal)
            }
            AstExpr::Sequence(_) => ExprKind::Sequence(
                expr.child_exprs()
                    .into_iter()
                    .map(|child| self.lower(source, child))
                    .collect(),
            ),
            AstExpr::Struct(node) => self.lower_struct(source, node),
            AstExpr::Name(node) => node
                .name(source)
                .map(|name| ExprKind::Name(name.to_owned()))
                .unwrap_or(ExprKind::Missing),
            AstExpr::CallableValue(_) => self.lower_callable_value(source, expr),
            AstExpr::Call(_) => self.lower_call(source, expr),
            AstExpr::Field(node) => self.lower_field(source, expr, node.field_name(source)),
            AstExpr::Index(_) => self.lower_index(source, expr),
            AstExpr::Let(node) => self.lower_let(source, expr, node.name(source)),
            AstExpr::Assign(_) => self.lower_assign(source, expr),
            AstExpr::Unary(_) => self.lower_unary_op(source, expr),
            AstExpr::Binary(_) => self.lower_binary_op(source, expr),
            AstExpr::Propagate(_) => self.lower_unary(source, expr, ExprKind::Propagate),
            AstExpr::If(_) => self.lower_if(source, expr),
            AstExpr::Match(node) => self.lower_match(source, expr, node),
            AstExpr::While(_) => self.lower_while(source, expr),
            AstExpr::Loop(_) => self.lower_unary(source, expr, ExprKind::Loop),
            AstExpr::Break(_) => ExprKind::Break,
            AstExpr::Continue(_) => ExprKind::Continue,
            AstExpr::Return(_) => ExprKind::Return(
                expr.child_exprs()
                    .into_iter()
                    .next()
                    .map(|inner| Box::new(self.lower(source, inner))),
            ),
            AstExpr::Spawn(_) => self.lower_unary(source, expr, ExprKind::Spawn),
            AstExpr::Panic(_) => ExprKind::Panic(
                expr.child_exprs()
                    .into_iter()
                    .map(|arg| self.lower(source, arg))
                    .collect(),
            ),
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

    fn lower_group(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let Some(inner) = expr.child_exprs().into_iter().next() else {
            return ExprKind::Missing;
        };

        self.lower(source, inner).kind
    }

    fn lower_callable_value(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let Some(body) = expr.child_exprs().into_iter().next() else {
            return ExprKind::Missing;
        };

        ExprKind::CallableValue {
            params: callable_params(source, expr.syntax()),
            body: Box::new(self.lower(source, body)),
        }
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

    fn lower_let(&mut self, source: &str, expr: AstExpr<'_>, name: Option<&str>) -> ExprKind {
        ExprKind::Let {
            name: binding_name(name),
            shape: first_shape(expr.syntax()).map(|shape| lower_shape(source, shape)),
            value: expr
                .child_exprs()
                .into_iter()
                .next()
                .map(|value| Box::new(self.lower(source, value))),
        }
    }

    fn lower_assign(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let (Some(target), Some(value)) = (children.next(), children.next()) else {
            return ExprKind::Missing;
        };

        ExprKind::Assign {
            target: Box::new(self.lower(source, target)),
            value: Box::new(self.lower(source, value)),
        }
    }

    fn lower_unary_op(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let Some(inner) = expr.child_exprs().into_iter().next() else {
            return ExprKind::Missing;
        };
        let Some(op) = unary_op(expr.syntax()) else {
            return ExprKind::Missing;
        };

        ExprKind::Unary {
            op,
            expr: Box::new(self.lower(source, inner)),
        }
    }

    fn lower_binary_op(&mut self, source: &str, expr: AstExpr<'_>) -> ExprKind {
        let mut children = expr.child_exprs().into_iter();
        let (Some(lhs), Some(rhs)) = (children.next(), children.next()) else {
            return ExprKind::Missing;
        };
        let Some(op) = binary_op(expr.syntax()) else {
            return ExprKind::Missing;
        };

        ExprKind::Binary {
            op,
            lhs: Box::new(self.lower(source, lhs)),
            rhs: Box::new(self.lower(source, rhs)),
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

    fn alloc_id(&mut self) -> ExprId {
        let id = ExprId(self.next_id);
        self.next_id += 1;
        id
    }
}

fn binding_name(name: Option<&str>) -> Option<String> {
    name.filter(|name| *name != "_").map(str::to_owned)
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

fn callable_params(source: &str, node: &CstNode) -> Vec<ExprParam> {
    node.children
        .iter()
        .find_map(|child| match child {
            CstElement::Node(node) => ParamList::cast(node),
            CstElement::Token(_) => None,
        })
        .into_iter()
        .flat_map(ParamList::params)
        .map(|param| lower_expr_param(source, param))
        .collect()
}

fn lower_expr_param(source: &str, param: CallableParam<'_>) -> ExprParam {
    ExprParam {
        name: param.name(source).map(str::to_owned),
        span: param.syntax().span,
        shape: param
            .shape_annotation()
            .map(|shape| lower_shape(source, shape)),
    }
}

fn first_shape<'tree>(node: &'tree CstNode) -> Option<AstShape<'tree>> {
    node.children.iter().find_map(|child| match child {
        CstElement::Node(node) => AstShape::cast(node),
        CstElement::Token(_) => None,
    })
}

fn unary_op(node: &CstNode) -> Option<UnaryOp> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) => match token.kind {
            TokenKind::KeywordNot => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Tilde => Some(UnaryOp::BitNot),
            _ => None,
        },
        CstElement::Node(_) => None,
    })
}

fn binary_op(node: &CstNode) -> Option<BinaryOp> {
    let mut saw_is = false;
    let mut saw_not = false;
    let mut simple_op = None;

    for child in &node.children {
        let CstElement::Token(token) = child else {
            continue;
        };

        match token.kind {
            TokenKind::KeywordIs => saw_is = true,
            TokenKind::KeywordNot => saw_not = true,
            TokenKind::KeywordOr => simple_op = Some(BinaryOp::Or),
            TokenKind::KeywordAnd => simple_op = Some(BinaryOp::And),
            TokenKind::EqualEqual => simple_op = Some(BinaryOp::Equal),
            TokenKind::TildeEqual => simple_op = Some(BinaryOp::NotEqual),
            TokenKind::Less => simple_op = Some(BinaryOp::Less),
            TokenKind::LessEqual => simple_op = Some(BinaryOp::LessEqual),
            TokenKind::Greater => simple_op = Some(BinaryOp::Greater),
            TokenKind::GreaterEqual => simple_op = Some(BinaryOp::GreaterEqual),
            TokenKind::Pipe => simple_op = Some(BinaryOp::BitOr),
            TokenKind::Caret => simple_op = Some(BinaryOp::BitXor),
            TokenKind::Amp => simple_op = Some(BinaryOp::BitAnd),
            TokenKind::ShiftLeft => simple_op = Some(BinaryOp::ShiftLeft),
            TokenKind::ShiftRight => simple_op = Some(BinaryOp::ShiftRight),
            TokenKind::DotDot => simple_op = Some(BinaryOp::RangeExclusive),
            TokenKind::DotDotEqual => simple_op = Some(BinaryOp::RangeInclusive),
            TokenKind::Plus => simple_op = Some(BinaryOp::Add),
            TokenKind::Minus => simple_op = Some(BinaryOp::Sub),
            TokenKind::Star => simple_op = Some(BinaryOp::Mul),
            TokenKind::Slash => simple_op = Some(BinaryOp::Div),
            TokenKind::Percent => simple_op = Some(BinaryOp::Rem),
            _ => {}
        }
    }

    if saw_is && saw_not {
        Some(BinaryOp::IsNot)
    } else if saw_is {
        Some(BinaryOp::Is)
    } else {
        simple_op
    }
}
