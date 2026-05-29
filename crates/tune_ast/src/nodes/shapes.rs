use tune_syntax::{CstNode, SyntaxKind};

use crate::AstNode;

#[derive(Debug, Clone, Copy)]
pub enum Shape<'tree> {
    Named(NamedShape<'tree>),
    Sequence(SequenceShape<'tree>),
    Generic(GenericShape<'tree>),
    Structural(StructuralShape<'tree>),
    Tuple(TupleShape<'tree>),
    Optional(OptionalShape<'tree>),
    Union(UnionShape<'tree>),
    Callable(CallableShape<'tree>),
}

impl<'tree> Shape<'tree> {
    #[must_use]
    pub fn cast(node: &'tree CstNode) -> Option<Self> {
        match node.kind {
            SyntaxKind::Shape => NamedShape::cast(node).map(Self::Named),
            SyntaxKind::SequenceShape => SequenceShape::cast(node).map(Self::Sequence),
            SyntaxKind::GenericShape => GenericShape::cast(node).map(Self::Generic),
            SyntaxKind::StructuralShape => StructuralShape::cast(node).map(Self::Structural),
            SyntaxKind::TupleShape => TupleShape::cast(node).map(Self::Tuple),
            SyntaxKind::OptionalShape => OptionalShape::cast(node).map(Self::Optional),
            SyntaxKind::UnionShape => UnionShape::cast(node).map(Self::Union),
            SyntaxKind::CallableShape => CallableShape::cast(node).map(Self::Callable),
            _ => None,
        }
    }

    #[must_use]
    pub fn syntax(self) -> &'tree CstNode {
        match self {
            Self::Named(node) => node.syntax(),
            Self::Sequence(node) => node.syntax(),
            Self::Generic(node) => node.syntax(),
            Self::Structural(node) => node.syntax(),
            Self::Tuple(node) => node.syntax(),
            Self::Optional(node) => node.syntax(),
            Self::Union(node) => node.syntax(),
            Self::Callable(node) => node.syntax(),
        }
    }
}

macro_rules! shape_node {
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

shape_node!(NamedShape, SyntaxKind::Shape);
shape_node!(SequenceShape, SyntaxKind::SequenceShape);
shape_node!(GenericShape, SyntaxKind::GenericShape);
shape_node!(StructuralShape, SyntaxKind::StructuralShape);
shape_node!(TupleShape, SyntaxKind::TupleShape);
shape_node!(OptionalShape, SyntaxKind::OptionalShape);
shape_node!(UnionShape, SyntaxKind::UnionShape);
shape_node!(CallableShape, SyntaxKind::CallableShape);
