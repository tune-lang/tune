#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Root,
    LetDecl,
    CallableDecl,
    StructDecl,
    EnumDecl,
    TagDecl,
    Expr,
    Pattern,
    Error,
}

#[derive(Debug, Clone)]
pub struct CstNode {
    pub kind: SyntaxKind,
    pub children: Vec<CstNode>,
}
