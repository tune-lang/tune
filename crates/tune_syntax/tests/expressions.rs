use tune_syntax::{CstElement, SyntaxKind, parse};

#[test]
fn parses_expression_nodes_in_declaration_bodies() {
    let parsed = parse(
        r#"
let value = items[0].name!
let task = spawn fetch()
let looped = for item in items { handle(item) }
let numbers = [1, 2, 3]
let block = { let x = 1; x = x; return x }
let grouped = (1 + 2)
let ops = (not value and other) or (other is not none)
let inline_branch = if count == 1 => "item" else "items"
let branched = if ready { Ok(value) } elif waiting { Error("wait") } else { panic("bad") }
let matched = match result { Ok(value) => value; Error(err) => panic(err); else none }
let matched_block = match result { Ok(value) { value } else { none } }
let matched_shape = match duck { { quack(): String } => duck.quack(); else none }
let repeated = while ready { continue }
let forever = loop { break }
"#,
    );
    let kinds = nested_node_kinds(&parsed.cst);

    assert!(kinds.contains(&SyntaxKind::IndexExpr));
    assert!(kinds.contains(&SyntaxKind::FieldExpr));
    assert!(kinds.contains(&SyntaxKind::PropagateExpr));
    assert!(kinds.contains(&SyntaxKind::SpawnExpr));
    assert!(kinds.contains(&SyntaxKind::CallExpr));
    assert!(kinds.contains(&SyntaxKind::ForExpr));
    assert!(kinds.contains(&SyntaxKind::Block));
    assert!(kinds.contains(&SyntaxKind::SequenceExpr));
    assert!(kinds.contains(&SyntaxKind::TupleExpr));
    assert!(kinds.contains(&SyntaxKind::LetExpr));
    assert!(kinds.contains(&SyntaxKind::AssignExpr));
    assert!(kinds.contains(&SyntaxKind::ReturnExpr));
    assert!(kinds.contains(&SyntaxKind::UnaryExpr));
    assert!(kinds.contains(&SyntaxKind::BinaryExpr));
    assert!(kinds.contains(&SyntaxKind::IfExpr));
    assert!(kinds.contains(&SyntaxKind::MatchExpr));
    assert!(kinds.contains(&SyntaxKind::MatchArm));
    assert!(kinds.contains(&SyntaxKind::PatternList));
    assert!(kinds.contains(&SyntaxKind::StructuralPattern));
    assert!(kinds.contains(&SyntaxKind::StructuralRequirement));
    assert!(kinds.contains(&SyntaxKind::WhileExpr));
    assert!(kinds.contains(&SyntaxKind::LoopExpr));
    assert!(kinds.contains(&SyntaxKind::BreakExpr));
    assert!(kinds.contains(&SyntaxKind::ContinueExpr));
    assert!(kinds.contains(&SyntaxKind::PanicExpr));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn parses_tuple_expression_nodes() {
    let parsed = parse(r#"let pair = (10, "hello")"#);
    let kinds = nested_node_kinds(&parsed.cst);

    assert!(kinds.contains(&SyntaxKind::TupleExpr));
    assert_eq!(
        kinds
            .iter()
            .filter(|kind| **kind == SyntaxKind::LiteralExpr)
            .count(),
        2
    );
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn reports_missing_block_expression_separator() {
    let parsed = parse("let block = { a b }");

    assert_eq!(parsed.diagnostics.len(), 1);
    assert_eq!(
        parsed.diagnostics[0].title,
        "expected `;` or newline between expressions"
    );
}

#[test]
fn reports_arrow_after_else_body_boundary() {
    let parsed = parse(r#"let label = if ready => "yes" else => "no""#);

    assert!(parsed.diagnostics.iter().any(|diagnostic| {
        diagnostic.title.contains("expected expression")
            || diagnostic.title.contains("expected `else` body")
    }));
}

#[test]
fn newline_after_if_body_separates_next_block_expression() {
    let parsed = parse(
        r#"
let pick = {
  if ready {
    return 1
  }
  2
}
"#,
    );

    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn newline_after_literal_initializer_separates_next_block_expression() {
    let parsed = parse(
        r#"
let pick = {
  let result = 0
  if ready { result = 1 } else { result = 2 }
  result
}
"#,
    );

    assert!(parsed.diagnostics.is_empty());
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
