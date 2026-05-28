use tune_ast::nodes::Shape as AstShape;
use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::pattern::{Pattern, PatternKind, StructuralRequirement, StructuralRequirementKind};

use super::super::shapes::lower_shape;
use super::ExprLowerer;

pub(super) fn lower_pattern(source: &str, node: &CstNode, lowerer: &mut ExprLowerer) -> Pattern {
    let Some(pattern) = node.children.iter().find_map(|child| match child {
        CstElement::Node(node) if node.kind == SyntaxKind::Pattern => Some(node),
        CstElement::Node(_) | CstElement::Token(_) => None,
    }) else {
        return Pattern {
            id: lowerer.alloc_id(),
            span: node.span,
            kind: PatternKind::Hole,
        };
    };

    lower_pattern_node(source, pattern, lowerer)
}

fn lower_pattern_node(source: &str, pattern: &CstNode, lowerer: &mut ExprLowerer) -> Pattern {
    if let Some(requirements) = structural_requirements(source, pattern, lowerer) {
        return Pattern {
            id: lowerer.alloc_id(),
            span: pattern.span,
            kind: PatternKind::StructuralShape(requirements),
        };
    }

    let name = pattern_name(source, pattern);
    let args = pattern_list(source, pattern, lowerer);

    let kind = match (name, args) {
        (Some("else"), None) => PatternKind::Else,
        (Some("_"), None) => PatternKind::Hole,
        (Some(name), Some(args)) => PatternKind::Variant {
            name: name.to_owned(),
            args,
        },
        (Some(name), None) => PatternKind::Binding(name.to_owned()),
        (None, Some(args)) if args.is_empty() => PatternKind::Unit,
        (None, Some(args)) => PatternKind::Tuple(args),
        (None, None) => PatternKind::Hole,
    };

    Pattern {
        id: lowerer.alloc_id(),
        span: pattern.span,
        kind,
    }
}

fn structural_requirements(
    source: &str,
    pattern: &CstNode,
    lowerer: &mut ExprLowerer,
) -> Option<Vec<StructuralRequirement>> {
    pattern.children.iter().find_map(|child| match child {
        CstElement::Node(node) if node.kind == SyntaxKind::StructuralPattern => Some(
            node.children
                .iter()
                .filter_map(|child| match child {
                    CstElement::Node(node) if node.kind == SyntaxKind::StructuralRequirement => {
                        structural_requirement(source, node, lowerer)
                    }
                    CstElement::Node(_) | CstElement::Token(_) => None,
                })
                .collect(),
        ),
        CstElement::Node(_) | CstElement::Token(_) => None,
    })
}

fn structural_requirement(
    source: &str,
    node: &CstNode,
    lowerer: &mut ExprLowerer,
) -> Option<StructuralRequirement> {
    let name = first_ident_text(node, source)?.to_owned();
    let shapes = child_shapes(source, node);
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
        StructuralRequirementKind::Callable { name, params, ret }
    } else {
        StructuralRequirementKind::Field {
            name,
            shape: shapes.into_iter().next(),
        }
    };

    Some(StructuralRequirement {
        id: lowerer.alloc_id(),
        span: node.span,
        kind,
    })
}

fn has_return_shape(node: &CstNode) -> bool {
    node.children
        .iter()
        .any(|child| matches!(child, CstElement::Token(token) if token.kind == TokenKind::Colon))
}

fn child_shapes(source: &str, node: &CstNode) -> Vec<crate::shape::ShapeExpr> {
    node.children
        .iter()
        .flat_map(|child| match child {
            CstElement::Node(child) if child.kind == SyntaxKind::ShapeList => child
                .children
                .iter()
                .filter_map(|item| match item {
                    CstElement::Node(item) => AstShape::cast(item),
                    CstElement::Token(_) => None,
                })
                .collect(),
            CstElement::Node(child) => AstShape::cast(child).into_iter().collect(),
            CstElement::Token(_) => Vec::new(),
        })
        .map(|shape| lower_shape(source, shape))
        .collect()
}

fn pattern_name<'src>(source: &'src str, pattern: &CstNode) -> Option<&'src str> {
    pattern.children.iter().find_map(|child| match child {
        CstElement::Token(token)
            if matches!(
                token.kind,
                TokenKind::Ident
                    | TokenKind::KeywordSelf
                    | TokenKind::KeywordOk
                    | TokenKind::KeywordError
                    | TokenKind::KeywordElse
            ) =>
        {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(_) | CstElement::Token(_) => None,
    })
}

fn first_ident_text<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token) if token.kind == TokenKind::Ident => {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(node) => first_ident_text(node, source),
        CstElement::Token(_) => None,
    })
}

fn pattern_list(
    source: &str,
    pattern: &CstNode,
    lowerer: &mut ExprLowerer,
) -> Option<Vec<Pattern>> {
    pattern.children.iter().find_map(|child| match child {
        CstElement::Node(node) if node.kind == SyntaxKind::PatternList => Some(
            node.children
                .iter()
                .filter_map(|child| match child {
                    CstElement::Node(node) if node.kind == SyntaxKind::Pattern => {
                        Some(lower_pattern_node(source, node, lowerer))
                    }
                    CstElement::Node(_) | CstElement::Token(_) => None,
                })
                .collect(),
        ),
        CstElement::Node(_) | CstElement::Token(_) => None,
    })
}
