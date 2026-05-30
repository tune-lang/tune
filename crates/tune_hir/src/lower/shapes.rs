use tune_ast::AstNode;
use tune_ast::nodes::Shape as AstShape;
use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::shape::{
    ShapeExpr, ShapeExprKind, StructuralShapeRequirement, StructuralShapeRequirementKind,
};

pub(super) fn lower_shape(source: &str, shape: AstShape<'_>) -> ShapeExpr {
    let span = shape.syntax().span;
    let kind = match shape {
        AstShape::Named(node) => shape_name(node.syntax(), source)
            .map(ShapeExprKind::Named)
            .unwrap_or(ShapeExprKind::Missing),
        AstShape::Generic(node) => shape_name(node.syntax(), source)
            .map(|name| ShapeExprKind::Generic {
                name,
                args: generic_shape_args(source, node.syntax()),
            })
            .unwrap_or(ShapeExprKind::Missing),
        AstShape::Structural(node) => ShapeExprKind::Structural(
            node.syntax()
                .children
                .iter()
                .filter_map(|child| match child {
                    CstElement::Node(node) if node.kind == SyntaxKind::StructuralRequirement => {
                        structural_shape_requirement(source, node)
                    }
                    CstElement::Node(_) | CstElement::Token(_) => None,
                })
                .collect(),
        ),
        AstShape::Sequence(node) => child_shapes(node.syntax())
            .into_iter()
            .next()
            .map(|shape| ShapeExprKind::Sequence(Box::new(lower_shape(source, shape))))
            .unwrap_or(ShapeExprKind::Missing),
        AstShape::Tuple(node) => ShapeExprKind::Tuple(shape_list_items(source, node.syntax())),
        AstShape::Optional(node) => child_shapes(node.syntax())
            .into_iter()
            .next()
            .map(|shape| ShapeExprKind::Optional(Box::new(lower_shape(source, shape))))
            .unwrap_or(ShapeExprKind::Missing),
        AstShape::Union(node) => ShapeExprKind::Union(
            child_shapes(node.syntax())
                .into_iter()
                .map(|shape| lower_shape(source, shape))
                .collect(),
        ),
        AstShape::Callable(node) => lower_callable_shape(source, node.syntax()),
    };

    ShapeExpr { kind, span }
}

fn structural_shape_requirement(
    source: &str,
    node: &CstNode,
) -> Option<StructuralShapeRequirement> {
    let name = first_shape_name(node, source)?.to_owned();
    let shapes = child_shapes(node)
        .into_iter()
        .map(|shape| lower_shape(source, shape))
        .collect::<Vec<_>>();
    let has_param_list = node.children.iter().any(
        |child| matches!(child, CstElement::Token(token) if token.kind == TokenKind::LeftParen),
    );

    let kind = if has_param_list {
        let mut params = shapes;
        let ret = if has_return_shape(node) {
            params.pop()
        } else {
            None
        };
        StructuralShapeRequirementKind::Callable { params, ret }
    } else {
        StructuralShapeRequirementKind::Field {
            shape: shapes.into_iter().next(),
        }
    };

    Some(StructuralShapeRequirement {
        name,
        span: node.span,
        kind,
    })
}

fn has_return_shape(node: &CstNode) -> bool {
    node.children
        .iter()
        .any(|child| matches!(child, CstElement::Token(token) if token.kind == TokenKind::Colon))
}

fn lower_callable_shape(source: &str, node: &CstNode) -> ShapeExprKind {
    let mut children = child_shapes(node);
    let params = children
        .first()
        .map(|shape| shape_list_items(source, shape.syntax()))
        .unwrap_or_default();
    let ret = children
        .pop()
        .map(|shape| lower_shape(source, shape))
        .unwrap_or(ShapeExpr {
            kind: ShapeExprKind::Missing,
            span: node.span,
        });

    ShapeExprKind::Callable {
        params,
        ret: Box::new(ret),
    }
}

fn shape_list_items(source: &str, node: &CstNode) -> Vec<ShapeExpr> {
    node.children
        .iter()
        .flat_map(|child| match child {
            CstElement::Node(node) if node.kind == SyntaxKind::ShapeList => child_shapes(node),
            CstElement::Node(node) => AstShape::cast(node).into_iter().collect(),
            CstElement::Token(_) => Vec::new(),
        })
        .map(|shape| lower_shape(source, shape))
        .collect()
}

fn generic_shape_args(source: &str, node: &CstNode) -> Vec<ShapeExpr> {
    node.children
        .iter()
        .find_map(|child| match child {
            CstElement::Node(node) if node.kind == SyntaxKind::ShapeList => {
                Some(shape_list_items(source, node))
            }
            CstElement::Node(_) | CstElement::Token(_) => None,
        })
        .unwrap_or_default()
}

fn child_shapes<'tree>(node: &'tree CstNode) -> Vec<AstShape<'tree>> {
    node.children
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(node) => AstShape::cast(node),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn first_shape_name<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token)
            if matches!(token.kind, TokenKind::Ident | TokenKind::KeywordNever) =>
        {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(node) => first_shape_name(node, source),
        CstElement::Token(_) => None,
    })
}

fn shape_name(node: &CstNode, source: &str) -> Option<String> {
    let mut name = String::new();
    for child in &node.children {
        match child {
            CstElement::Token(token)
                if matches!(token.kind, TokenKind::Ident | TokenKind::KeywordNever) =>
            {
                let start = token.span.start.get() as usize;
                let end = token.span.end.get() as usize;
                if !name.is_empty() {
                    name.push('.');
                }
                name.push_str(source.get(start..end)?);
            }
            CstElement::Node(node) => return shape_name(node, source),
            CstElement::Token(_) => {}
        }
    }
    (!name.is_empty()).then_some(name)
}
