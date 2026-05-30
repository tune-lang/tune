#[test]
fn lowers_top_level_declarations() {
    let source = r#"
import "std/json"
-- Tool docs.
tag tool {}
-/
Run docs.
Can be multiline.
/-
pub let run(input) = input
struct Counter {}
enum Result {}
let value = 1
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items.len(), 6);
    assert!(matches!(
        module.items[0].kind,
        tune_hir::item::ItemKind::Import
    ));
    assert_eq!(module.items[0].name.as_deref(), Some("std/json"));
    assert_eq!(module.items[1].name.as_deref(), Some("tool"));
    assert_eq!(module.items[1].doc.as_deref(), Some("Tool docs."));
    assert!(matches!(
        module.items[2].kind,
        tune_hir::item::ItemKind::CallableDecl
    ));
    assert_eq!(
        module.items[2].visibility,
        tune_hir::item::Visibility::Public
    );
    assert_eq!(
        module.items[2].doc.as_deref(),
        Some("Run docs.\nCan be multiline.")
    );
    assert_eq!(module.items[5].name.as_deref(), Some("value"));
    assert_eq!(module.items[5].id, tune_hir::HirId(5));
}

#[test]
fn lowers_top_level_expression_items() -> Result<(), &'static str> {
    let source = "let message = \"hello\"\nprint(message)";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items.len(), 2);
    assert_eq!(module.items[1].kind, tune_hir::item::ItemKind::Expr);
    assert_eq!(module.items[1].name, None);
    assert!(matches!(
        module.items[1]
            .body
            .as_ref()
            .ok_or("expected top-level expression body")?
            .kind,
        tune_hir::expr::ExprKind::Call { .. }
    ));

    Ok(())
}

#[test]
fn lowers_import_selectors() -> Result<(), &'static str> {
    let source = r#"
import "net/http".client
import "std/json".{parse, stringify}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert!(matches!(
        module.items[0].import.as_ref().map(|import| &import.selector),
        Some(tune_hir::item::ImportSelector::Member(name)) if name == "client"
    ));
    assert!(matches!(
        module.items[1].import.as_ref().map(|import| &import.selector),
        Some(tune_hir::item::ImportSelector::Members(names))
            if names == &vec!["parse".to_owned(), "stringify".to_owned()]
    ));

    Ok(())
}

#[test]
fn lowers_underscore_binding_names_as_absent() -> Result<(), &'static str> {
    let source = "let _ = 1\nlet value = { let _ = 2; 3 }";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items[0].name, None);
    let body = module.items[1].body.as_ref().ok_or("expected value body")?;
    let tune_hir::expr::ExprKind::Block(exprs) = &body.kind else {
        return Err("expected block");
    };
    assert!(matches!(
        exprs[0].kind,
        tune_hir::expr::ExprKind::Let { name: None, .. }
    ));

    Ok(())
}

#[test]
fn lowers_is_phrases_to_canonical_equality_ops() -> Result<(), &'static str> {
    let source = "let value = { 1 is 1; 1 is not 2 }";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let body = module.items[0].body.as_ref().ok_or("expected value body")?;
    let tune_hir::expr::ExprKind::Block(exprs) = &body.kind else {
        return Err("expected block");
    };

    assert!(matches!(
        exprs[0].kind,
        tune_hir::expr::ExprKind::Binary {
            op: tune_hir::expr::BinaryOp::Equal,
            ..
        }
    ));
    assert!(matches!(
        exprs[1].kind,
        tune_hir::expr::ExprKind::Binary {
            op: tune_hir::expr::BinaryOp::NotEqual,
            ..
        }
    ));

    Ok(())
}

#[test]
fn lowers_tuple_expressions_and_normalizes_grouping() -> Result<(), &'static str> {
    let source = r#"let pair = (10, "hello")
let grouped = (10)"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    let pair = module.items[0].body.as_ref().ok_or("expected pair body")?;
    assert!(matches!(
        pair.kind,
        tune_hir::expr::ExprKind::Tuple(ref items) if items.len() == 2
    ));

    let grouped = module.items[1]
        .body
        .as_ref()
        .ok_or("expected grouped body")?;
    assert!(matches!(
        grouped.kind,
        tune_hir::expr::ExprKind::Literal(tune_hir::expr::LiteralKind::Int(_))
    ));

    Ok(())
}

#[test]
fn lowers_none_match_pattern_as_literal_pattern() -> Result<(), &'static str> {
    let source = "let result = match value { none => 1; else 2 }";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let body = module.items[0].body.as_ref().ok_or("expected body")?;
    let tune_hir::expr::ExprKind::Match { arms, .. } = &body.kind else {
        return Err("expected match expression");
    };

    assert!(matches!(
        arms[0].pattern.kind,
        tune_hir::pattern::PatternKind::None
    ));
    assert!(matches!(
        arms[1].pattern.kind,
        tune_hir::pattern::PatternKind::Else
    ));

    Ok(())
}

#[test]
fn lowers_struct_field_defaults() -> Result<(), &'static str> {
    let source = r#"
struct Counter {
  value: Int = 0
  inferred = 1
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let item = module.items.first().ok_or("expected struct item")?;

    assert_eq!(item.fields.len(), 2);
    assert!(item.fields[0].shape.is_some());
    assert!(item.fields[0].default.is_some());
    assert!(item.fields[1].shape.is_none());
    assert!(item.fields[1].default.is_some());

    Ok(())
}

#[test]
fn lowers_compound_assignment_to_assignment_with_binary_value() -> Result<(), &'static str> {
    let source = "let result = { let value: Int = 1; value += 2 }";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let body = module.items[0].body.as_ref().ok_or("expected body")?;
    let tune_hir::expr::ExprKind::Block(exprs) = &body.kind else {
        return Err("expected block");
    };
    let tune_hir::expr::ExprKind::Assign { value, .. } = &exprs[1].kind else {
        return Err("expected assignment");
    };

    assert!(matches!(
        value.kind,
        tune_hir::expr::ExprKind::Binary {
            op: tune_hir::expr::BinaryOp::Add,
            ..
        }
    ));

    Ok(())
}

#[test]
fn lowers_callable_type_param_structural_constraints() -> Result<(), &'static str> {
    let source = r#"let quack<T: { quack(): String }>(duck: T): String = duck.quack()"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let item = module.items.first().ok_or("expected callable item")?;
    let type_param = item.type_params.first().ok_or("expected type param")?;
    let constraint = type_param
        .constraint
        .as_ref()
        .ok_or("expected type param constraint")?;

    assert!(matches!(
        constraint.kind,
        tune_hir::shape::ShapeExprKind::Structural(ref requirements)
            if requirements.len() == 1
    ));

    Ok(())
}

#[test]
fn lowers_string_literals_to_segments_and_trims_multiline() -> Result<(), &'static str> {
    let source = "let message = \"value is {value}\"\nlet text = \"\"\"\n  one\n  two\n  \"\"\"";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let message = module.items[0].body.as_ref().ok_or("expected message")?;
    let text = module.items[1].body.as_ref().ok_or("expected text")?;

    let tune_hir::expr::ExprKind::Literal(tune_hir::expr::LiteralKind::String(message)) =
        &message.kind
    else {
        return Err("expected string literal");
    };
    assert_eq!(message.parts.len(), 2);
    let tune_hir::expr::StringPart::Interpolation(expr) = &message.parts[1] else {
        return Err("expected interpolation expression");
    };
    assert!(matches!(
        expr.kind,
        tune_hir::expr::ExprKind::Name(ref name) if name == "value"
    ));

    let tune_hir::expr::ExprKind::Literal(tune_hir::expr::LiteralKind::String(text)) = &text.kind
    else {
        return Err("expected multiline string literal");
    };
    assert_eq!(text.plain_text().as_deref(), Some("one\ntwo"));

    Ok(())
}

#[test]
fn lowers_shape_annotations_to_hir_shape_exprs() -> Result<(), &'static str> {
    let source = "let value: [Int | String]? = none";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let shape = module.items[0]
        .shape
        .as_ref()
        .ok_or("expected shape annotation")?;

    assert!(matches!(
        shape.kind,
        tune_hir::shape::ShapeExprKind::Optional(_)
    ));
    let tune_hir::shape::ShapeExprKind::Optional(inner) = &shape.kind else {
        return Err("expected optional shape");
    };
    let tune_hir::shape::ShapeExprKind::Sequence(element) = &inner.kind else {
        return Err("optional should wrap sequence");
    };
    assert!(matches!(
        element.kind,
        tune_hir::shape::ShapeExprKind::Union(_)
    ));

    Ok(())
}

#[test]
fn lowers_callable_signature_params_and_return_shape() {
    let source = r#"let parse(text: String, strict: Bool): Result -/
Parse docs.
/- = text"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let item = &module.items[0];

    assert_eq!(item.name.as_deref(), Some("parse"));
    assert_eq!(item.doc.as_deref(), Some("Parse docs."));
    assert_eq!(item.params.len(), 2);
    assert_eq!(item.params[0].id.owner, item.id);
    assert_eq!(item.params[0].id.index, 0);
    assert_eq!(item.params[0].name.as_deref(), Some("text"));
    assert!(item.params[0].shape.is_some());
    assert_eq!(item.params[1].name.as_deref(), Some("strict"));
    assert!(item.params[1].shape.is_some());
    assert!(item.shape.is_some());
}

#[test]
fn lowers_generic_shape_annotations() -> Result<(), &'static str> {
    let source = "let parse(text: String): Result<Config, ParseError> = text";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let shape = module.items[0]
        .shape
        .as_ref()
        .ok_or("expected return shape")?;

    let tune_hir::shape::ShapeExprKind::Generic { name, args } = &shape.kind else {
        return Err("expected generic shape");
    };

    assert_eq!(name, "Result");
    assert_eq!(args.len(), 2);

    Ok(())
}

#[test]
fn lowers_declaration_type_params() {
    let source = r#"
struct Box<T> { value: T }
enum Response<T, E> { Ok(T) Error(E) }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items[0].type_params[0].name.as_deref(), Some("T"));
    assert_eq!(module.items[0].type_params[0].id.owner, module.items[0].id);
    assert_eq!(
        module.items[0].type_params[0].id.kind,
        tune_hir::MemberKind::TypeParam
    );
    assert_eq!(
        module.items[1]
            .type_params
            .iter()
            .filter_map(|param| param.name.as_deref())
            .collect::<Vec<_>>(),
        ["T", "E"]
    );
}

#[test]
fn lowers_declaration_body_members() {
    let source = r#"
struct User {
  -- Name docs.
  name: String
  age: Int
  rename(value: String): Unit = value
  [items] = items
  User[index: Size]: String = name
}
enum LoadResult {
  Ok(User)
  Error(String)
}
tag tool {
  title: String
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items[0].fields.len(), 2);
    assert_eq!(module.items[0].struct_members.len(), 5);
    assert_eq!(module.items[0].fields[0].id.owner, module.items[0].id);
    assert_eq!(module.items[0].fields[0].id.index, 0);
    assert_eq!(module.items[0].fields[0].name.as_deref(), Some("name"));
    assert_eq!(module.items[0].fields[0].doc.as_deref(), Some("Name docs."));
    assert!(module.items[0].fields[0].shape.is_some());
    assert!(module.items[0].struct_members.iter().any(|member| {
        matches!(member, tune_hir::item::StructMember::Callable(callable) if callable.name.as_deref() == Some("rename") && callable.body.is_some())
    }));
    assert!(module.items[0].struct_members.iter().any(|member| {
        matches!(member, tune_hir::item::StructMember::SequenceMaterializer(materializer) if materializer.param_name.as_deref() == Some("items") && materializer.body.is_some())
    }));
    assert!(module.items[0].struct_members.iter().any(|member| {
        matches!(member, tune_hir::item::StructMember::IndexAccess(access) if access.receiver_name.as_deref() == Some("User") && access.index_param_name.as_deref() == Some("index") && access.index_shape.is_some() && access.result_shape.is_some())
    }));
    assert_eq!(module.items[1].variants.len(), 2);
    assert_eq!(module.items[1].variants[0].id.owner, module.items[1].id);
    assert_eq!(module.items[1].variants[0].id.index, 0);
    assert_eq!(module.items[1].variants[0].name.as_deref(), Some("Ok"));
    assert_eq!(module.items[1].variants[0].payload.len(), 1);
    assert_eq!(module.items[2].fields.len(), 1);
    assert_eq!(module.items[2].fields[0].name.as_deref(), Some("title"));
}

#[test]
fn lowers_tag_applications_to_hir_items() {
    let source = r#"
tag tool {}
let capability = 1
@tool(path: "/search", capability = capability)
pub let search(query) = query
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items[2].name.as_deref(), Some("search"));
    assert_eq!(module.items[2].tags.len(), 1);
    assert_eq!(module.items[2].tags[0].name, "tool");
    assert!(module.items[2].tags[0].span.is_some());
    assert_eq!(module.items[2].tags[0].args.len(), 2);
    assert_eq!(
        module.items[2].tags[0].args[0].name.as_deref(),
        Some("path")
    );
    assert_eq!(
        module.items[2].tags[0].args[1].name.as_deref(),
        Some("capability")
    );
}
