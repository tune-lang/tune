use crate::{Token, TokenKind};
use tune_diagnostics::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Root,
    Module,
    LetDecl,
    CallableDecl,
    CallableValue,
    StructDecl,
    EnumDecl,
    TagDecl,
    TagApplication,
    ImportDecl,
    PubDecl,
    Block,
    ParamList,
    Param,
    FieldDecl,
    VariantDecl,
    Shape,
    ShapeList,
    TupleShape,
    SequenceShape,
    OptionalShape,
    UnionShape,
    CallableShape,
    Expr,
    LiteralExpr,
    NameExpr,
    CallExpr,
    FieldExpr,
    IndexExpr,
    UnaryExpr,
    BinaryExpr,
    IfExpr,
    MatchExpr,
    LoopExpr,
    WhileExpr,
    ForExpr,
    SpawnExpr,
    ReturnExpr,
    BreakExpr,
    ContinueExpr,
    Pattern,
    PatternList,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CstNode {
    pub kind: SyntaxKind,
    pub span: Option<Span>,
    pub children: Vec<CstElement>,
}

impl CstNode {
    #[must_use]
    pub fn new(kind: SyntaxKind, children: Vec<CstElement>) -> Self {
        let span = covering_span(children.iter().filter_map(CstElement::span));
        Self {
            kind,
            span,
            children,
        }
    }

    #[must_use]
    pub const fn empty(kind: SyntaxKind) -> Self {
        Self {
            kind,
            span: None,
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CstElement {
    Node(CstNode),
    Token(Token),
}

impl CstElement {
    #[must_use]
    pub const fn node(node: CstNode) -> Self {
        Self::Node(node)
    }

    #[must_use]
    pub const fn token(token: Token) -> Self {
        Self::Token(token)
    }

    #[must_use]
    pub const fn span(&self) -> Option<Span> {
        match self {
            Self::Node(node) => node.span,
            Self::Token(token) => Some(token.span),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CstBuilder {
    stack: Vec<OpenNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Checkpoint {
    depth: usize,
    child_index: usize,
}

impl CstBuilder {
    #[must_use]
    pub fn new(root: SyntaxKind) -> Self {
        Self {
            stack: vec![OpenNode::new(root)],
        }
    }

    pub fn start_node(&mut self, kind: SyntaxKind) {
        self.stack.push(OpenNode::new(kind));
    }

    #[must_use]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            depth: self.stack.len(),
            child_index: self.stack.last().map_or(0, |node| node.children.len()),
        }
    }

    pub fn start_node_at(&mut self, checkpoint: Checkpoint, kind: SyntaxKind) {
        if checkpoint.depth != self.stack.len() {
            self.start_node(kind);
            return;
        }

        let current = self.current_mut();
        if checkpoint.child_index > current.children.len() {
            self.start_node(kind);
            return;
        }

        let children = current.children.split_off(checkpoint.child_index);
        self.stack.push(OpenNode { kind, children });
    }

    pub fn token(&mut self, token: Token) {
        self.current_mut().children.push(CstElement::Token(token));
    }

    pub fn finish_node(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }

        if let Some(open) = self.stack.pop() {
            let node = open.finish();
            self.current_mut().children.push(CstElement::Node(node));
        }
    }

    #[must_use]
    pub fn finish(mut self) -> CstNode {
        while self.stack.len() > 1 {
            self.finish_node();
        }

        match self.stack.pop() {
            Some(root) => root.finish(),
            None => CstNode::empty(SyntaxKind::Root),
        }
    }

    fn current_mut(&mut self) -> &mut OpenNode {
        if self.stack.is_empty() {
            self.stack.push(OpenNode::new(SyntaxKind::Root));
        }

        let index = self.stack.len() - 1;
        &mut self.stack[index]
    }
}

#[derive(Debug, Clone)]
struct OpenNode {
    kind: SyntaxKind,
    children: Vec<CstElement>,
}

impl OpenNode {
    const fn new(kind: SyntaxKind) -> Self {
        Self {
            kind,
            children: Vec::new(),
        }
    }

    fn finish(self) -> CstNode {
        CstNode::new(self.kind, self.children)
    }
}

fn covering_span(spans: impl IntoIterator<Item = Span>) -> Option<Span> {
    let mut spans = spans.into_iter();
    let first = spans.next()?;
    Some(spans.fold(first, merge_spans))
}

fn merge_spans(left: Span, right: Span) -> Span {
    if left.file != right.file {
        return left;
    }

    Span::new(
        left.file,
        left.start.min(right.start),
        left.end.max(right.end),
    )
}

#[must_use]
pub const fn is_trivia(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Whitespace | TokenKind::LineComment)
}
