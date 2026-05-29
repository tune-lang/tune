use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::AstNode;

use super::text::direct_ident_text;

#[derive(Debug, Clone, Copy)]
pub enum Expr<'tree> {
    Missing(&'tree CstNode),
    Group(GroupExpr<'tree>),
    Tuple(TupleExpr<'tree>),
    Literal(LiteralExpr<'tree>),
    Sequence(SequenceExpr<'tree>),
    Struct(StructExpr<'tree>),
    Name(NameExpr<'tree>),
    CallableValue(CallableValue<'tree>),
    Call(CallExpr<'tree>),
    Field(FieldExpr<'tree>),
    Index(IndexExpr<'tree>),
    Let(LetExpr<'tree>),
    Assign(AssignExpr<'tree>),
    Unary(UnaryExpr<'tree>),
    Binary(BinaryExpr<'tree>),
    Propagate(PropagateExpr<'tree>),
    If(IfExpr<'tree>),
    Match(MatchExpr<'tree>),
    While(WhileExpr<'tree>),
    Loop(LoopExpr<'tree>),
    Break(BreakExpr<'tree>),
    Continue(ContinueExpr<'tree>),
    Return(ReturnExpr<'tree>),
    For(ForExpr<'tree>),
    Spawn(SpawnExpr<'tree>),
    Panic(PanicExpr<'tree>),
    Block(BlockExpr<'tree>),
}

impl<'tree> Expr<'tree> {
    #[must_use]
    pub fn cast(node: &'tree CstNode) -> Option<Self> {
        match node.kind {
            SyntaxKind::LiteralExpr => LiteralExpr::cast(node).map(Self::Literal),
            SyntaxKind::TupleExpr => TupleExpr::cast(node).map(Self::Tuple),
            SyntaxKind::SequenceExpr => SequenceExpr::cast(node).map(Self::Sequence),
            SyntaxKind::StructExpr => StructExpr::cast(node).map(Self::Struct),
            SyntaxKind::NameExpr => NameExpr::cast(node).map(Self::Name),
            SyntaxKind::CallableValue => CallableValue::cast(node).map(Self::CallableValue),
            SyntaxKind::CallExpr => CallExpr::cast(node).map(Self::Call),
            SyntaxKind::FieldExpr => FieldExpr::cast(node).map(Self::Field),
            SyntaxKind::IndexExpr => IndexExpr::cast(node).map(Self::Index),
            SyntaxKind::LetExpr => LetExpr::cast(node).map(Self::Let),
            SyntaxKind::AssignExpr => AssignExpr::cast(node).map(Self::Assign),
            SyntaxKind::UnaryExpr => UnaryExpr::cast(node).map(Self::Unary),
            SyntaxKind::BinaryExpr => BinaryExpr::cast(node).map(Self::Binary),
            SyntaxKind::PropagateExpr => PropagateExpr::cast(node).map(Self::Propagate),
            SyntaxKind::IfExpr => IfExpr::cast(node).map(Self::If),
            SyntaxKind::MatchExpr => MatchExpr::cast(node).map(Self::Match),
            SyntaxKind::WhileExpr => WhileExpr::cast(node).map(Self::While),
            SyntaxKind::LoopExpr => LoopExpr::cast(node).map(Self::Loop),
            SyntaxKind::BreakExpr => BreakExpr::cast(node).map(Self::Break),
            SyntaxKind::ContinueExpr => ContinueExpr::cast(node).map(Self::Continue),
            SyntaxKind::ReturnExpr => ReturnExpr::cast(node).map(Self::Return),
            SyntaxKind::ForExpr => ForExpr::cast(node).map(Self::For),
            SyntaxKind::SpawnExpr => SpawnExpr::cast(node).map(Self::Spawn),
            SyntaxKind::PanicExpr => PanicExpr::cast(node).map(Self::Panic),
            SyntaxKind::Block => BlockExpr::cast(node).map(Self::Block),
            SyntaxKind::Expr => GroupExpr::cast(node).map(Self::Group),
            SyntaxKind::Error => Some(Self::Missing(node)),
            _ => None,
        }
    }

    #[must_use]
    pub fn syntax(self) -> &'tree CstNode {
        match self {
            Self::Missing(node) => node,
            Self::Group(node) => node.syntax(),
            Self::Tuple(node) => node.syntax(),
            Self::Literal(node) => node.syntax(),
            Self::Sequence(node) => node.syntax(),
            Self::Struct(node) => node.syntax(),
            Self::Name(node) => node.syntax(),
            Self::CallableValue(node) => node.syntax(),
            Self::Call(node) => node.syntax(),
            Self::Field(node) => node.syntax(),
            Self::Index(node) => node.syntax(),
            Self::Let(node) => node.syntax(),
            Self::Assign(node) => node.syntax(),
            Self::Unary(node) => node.syntax(),
            Self::Binary(node) => node.syntax(),
            Self::Propagate(node) => node.syntax(),
            Self::If(node) => node.syntax(),
            Self::Match(node) => node.syntax(),
            Self::While(node) => node.syntax(),
            Self::Loop(node) => node.syntax(),
            Self::Break(node) => node.syntax(),
            Self::Continue(node) => node.syntax(),
            Self::Return(node) => node.syntax(),
            Self::For(node) => node.syntax(),
            Self::Spawn(node) => node.syntax(),
            Self::Panic(node) => node.syntax(),
            Self::Block(node) => node.syntax(),
        }
    }

    #[must_use]
    pub fn child_exprs(self) -> Vec<Expr<'tree>> {
        child_exprs(self.syntax())
    }
}

macro_rules! expr_node {
    ($name:ident, $kind:expr) => {
        #[derive(Debug, Clone, Copy)]
        pub struct $name<'tree> {
            node: &'tree CstNode,
        }

        impl<'tree> AstNode<'tree> for $name<'tree> {
            const KIND: SyntaxKind = $kind;

            fn cast(node: &'tree CstNode) -> Option<Self> {
                (node.kind == Self::KIND).then_some(Self { node })
            }

            fn syntax(&self) -> &'tree CstNode {
                self.node
            }
        }
    };
}

expr_node!(LiteralExpr, SyntaxKind::LiteralExpr);
expr_node!(GroupExpr, SyntaxKind::Expr);
expr_node!(TupleExpr, SyntaxKind::TupleExpr);
expr_node!(SequenceExpr, SyntaxKind::SequenceExpr);
expr_node!(StructExpr, SyntaxKind::StructExpr);
expr_node!(StructFieldInit, SyntaxKind::StructFieldInit);
expr_node!(NameExpr, SyntaxKind::NameExpr);
expr_node!(CallableValue, SyntaxKind::CallableValue);
expr_node!(CallExpr, SyntaxKind::CallExpr);
expr_node!(FieldExpr, SyntaxKind::FieldExpr);
expr_node!(IndexExpr, SyntaxKind::IndexExpr);
expr_node!(LetExpr, SyntaxKind::LetExpr);
expr_node!(AssignExpr, SyntaxKind::AssignExpr);
expr_node!(UnaryExpr, SyntaxKind::UnaryExpr);
expr_node!(BinaryExpr, SyntaxKind::BinaryExpr);
expr_node!(PropagateExpr, SyntaxKind::PropagateExpr);
expr_node!(IfExpr, SyntaxKind::IfExpr);
expr_node!(MatchExpr, SyntaxKind::MatchExpr);
expr_node!(MatchArm, SyntaxKind::MatchArm);
expr_node!(WhileExpr, SyntaxKind::WhileExpr);
expr_node!(LoopExpr, SyntaxKind::LoopExpr);
expr_node!(BreakExpr, SyntaxKind::BreakExpr);
expr_node!(ContinueExpr, SyntaxKind::ContinueExpr);
expr_node!(ReturnExpr, SyntaxKind::ReturnExpr);
expr_node!(ForExpr, SyntaxKind::ForExpr);
expr_node!(SpawnExpr, SyntaxKind::SpawnExpr);
expr_node!(PanicExpr, SyntaxKind::PanicExpr);
expr_node!(BlockExpr, SyntaxKind::Block);

impl<'tree> LiteralExpr<'tree> {
    #[must_use]
    pub fn text(self, source: &str) -> Option<&str> {
        first_direct_token_text(self.node, source)
    }
}

impl<'tree> NameExpr<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source).or_else(|| direct_name_keyword_text(self.node, source))
    }
}

impl<'tree> StructExpr<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        self.node.children.iter().find_map(|child| match child {
            CstElement::Node(node) => NameExpr::cast(node).and_then(|name| name.name(source)),
            CstElement::Token(_) => None,
        })
    }

    #[must_use]
    pub fn fields(self) -> Vec<StructFieldInit<'tree>> {
        self.node
            .children
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(node) => StructFieldInit::cast(node),
                CstElement::Token(_) => None,
            })
            .collect()
    }
}

impl<'tree> StructFieldInit<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }

    #[must_use]
    pub fn value(self) -> Option<Expr<'tree>> {
        child_exprs(self.node).into_iter().next()
    }
}

impl<'tree> FieldExpr<'tree> {
    #[must_use]
    pub fn field_name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }
}

impl<'tree> LetExpr<'tree> {
    #[must_use]
    pub fn name(self, source: &str) -> Option<&str> {
        direct_ident_text(self.node, source)
    }
}

impl<'tree> BlockExpr<'tree> {
    #[must_use]
    pub fn exprs(self) -> Vec<Expr<'tree>> {
        child_exprs(self.node)
    }
}

impl<'tree> MatchExpr<'tree> {
    #[must_use]
    pub fn arms(self) -> Vec<MatchArm<'tree>> {
        self.node
            .children
            .iter()
            .filter_map(|child| match child {
                CstElement::Node(node) => MatchArm::cast(node),
                CstElement::Token(_) => None,
            })
            .collect()
    }
}

impl<'tree> MatchArm<'tree> {
    #[must_use]
    pub fn expr(self) -> Option<Expr<'tree>> {
        child_exprs(self.node).into_iter().next()
    }
}

fn child_exprs(node: &CstNode) -> Vec<Expr<'_>> {
    node.children
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(node) => Expr::cast(node),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn first_direct_token_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(_) => None,
    })
}

fn direct_name_keyword_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token)
            if matches!(
                token.kind,
                TokenKind::KeywordSelf | TokenKind::KeywordOk | TokenKind::KeywordError
            ) =>
        {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(_) | CstElement::Token(_) => None,
    })
}
