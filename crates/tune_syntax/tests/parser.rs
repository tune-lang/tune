use tune_syntax::{CstElement, SyntaxKind, parse};

#[test]
fn parses_top_level_declaration_nodes() {
    let parsed = parse(
        r#"
tag tool {}
struct Counter {}
enum Result {}
import "std"
let value = 1
"#,
    );

    let kinds = root_node_kinds(&parsed.cst);

    assert_eq!(
        kinds,
        [
            SyntaxKind::TagDecl,
            SyntaxKind::StructDecl,
            SyntaxKind::EnumDecl,
            SyntaxKind::ImportDecl,
            SyntaxKind::LetDecl,
        ]
    );
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn newline_ends_simple_declarations() {
    let parsed = parse(
        r#"
import "std"
let value = 1
let other = 2
"#,
    );

    assert_eq!(
        root_node_kinds(&parsed.cst),
        [
            SyntaxKind::ImportDecl,
            SyntaxKind::LetDecl,
            SyntaxKind::LetDecl,
        ]
    );
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn semicolon_ends_simple_declarations() {
    let parsed = parse(r#"import "std"; let value = 1; let other = 2"#);

    assert_eq!(
        root_node_kinds(&parsed.cst),
        [
            SyntaxKind::ImportDecl,
            SyntaxKind::LetDecl,
            SyntaxKind::LetDecl,
        ]
    );
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn parses_pub_as_visibility_wrapper() {
    let parsed = parse("pub let main() = {}");
    let pub_children = root_node_kinds(&parsed.cst);

    assert_eq!(pub_children, [SyntaxKind::PubDecl]);

    let nested = parsed
        .cst
        .children
        .iter()
        .filter_map(|element| match element {
            CstElement::Node(node) if node.kind == SyntaxKind::PubDecl => {
                Some(root_node_kinds(node))
            }
            CstElement::Node(_) | CstElement::Token(_) => None,
        })
        .flatten()
        .collect::<Vec<_>>();

    assert_eq!(nested, [SyntaxKind::CallableDecl]);
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn parses_tag_applications_as_top_level_attachments() {
    let parsed = parse(
        r#"
@tool
@route(path: "/search")
pub let search(query) = query
"#,
    );

    assert_eq!(
        root_node_kinds(&parsed.cst),
        [
            SyntaxKind::TagApplication,
            SyntaxKind::TagApplication,
            SyntaxKind::PubDecl,
        ]
    );
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn distinguishes_callable_declaration_from_callable_value_binding() {
    let callable_decl = parse("let f(x) = x");
    let callable_value = parse("let f = _(x) = x");

    assert_eq!(
        root_node_kinds(&callable_decl.cst),
        [SyntaxKind::CallableDecl]
    );
    assert_eq!(root_node_kinds(&callable_value.cst), [SyntaxKind::LetDecl]);
    assert_eq!(
        nested_node_kinds(&callable_value.cst),
        [SyntaxKind::LetDecl, SyntaxKind::CallableValue]
    );
}

#[test]
fn parses_shape_nodes_in_annotations() {
    let parsed = parse("let value: [Int | String]? = none");
    let kinds = nested_node_kinds(&parsed.cst);

    assert!(kinds.contains(&SyntaxKind::SequenceShape));
    assert!(kinds.contains(&SyntaxKind::UnionShape));
    assert!(kinds.contains(&SyntaxKind::OptionalShape));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn parses_callable_shape_nodes() {
    let parsed = parse("let f: (Int, String): Bool = handler");
    let kinds = nested_node_kinds(&parsed.cst);

    assert!(kinds.contains(&SyntaxKind::CallableShape));
    assert!(kinds.contains(&SyntaxKind::TupleShape));
    assert!(kinds.contains(&SyntaxKind::ShapeList));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn wraps_unexpected_top_level_token_in_error_node() {
    let parsed = parse("}");

    assert_eq!(root_node_kinds(&parsed.cst), [SyntaxKind::Error]);
    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(
        parsed.diagnostics[0].title,
        "expected top-level declaration"
    );
}

fn root_node_kinds(node: &tune_syntax::CstNode) -> Vec<SyntaxKind> {
    node.children
        .iter()
        .filter_map(|element| match element {
            CstElement::Node(node) => Some(node.kind),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn nested_node_kinds(node: &tune_syntax::CstNode) -> Vec<SyntaxKind> {
    let mut kinds = Vec::new();
    collect_node_kinds(node, &mut kinds);
    kinds
}

fn collect_node_kinds(node: &tune_syntax::CstNode, kinds: &mut Vec<SyntaxKind>) {
    for child in &node.children {
        if let CstElement::Node(node) = child {
            kinds.push(node.kind);
            collect_node_kinds(node, kinds);
        }
    }
}
