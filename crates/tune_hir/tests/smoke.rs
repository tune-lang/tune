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

    assert_eq!(module.items[0].type_params[0].name, "T");
    assert_eq!(
        module.items[1]
            .type_params
            .iter()
            .map(|param| param.name.as_str())
            .collect::<Vec<_>>(),
        ["T", "E"]
    );
}

#[test]
fn lowers_declaration_body_expressions() -> Result<(), &'static str> {
    let source = r#"
let value = items[0].name!
let task = spawn fetch()
let looped = for item in items { handle(item) }
let numbers = [1, 2, 3]
let callable = _(x: Int): Int = x
let block = { let x = 1; x = x; return x }
let grouped = (1 + 2)
let ops = (not value and other) or (other is not none)
let branched = if ready { Ok(value) } elif waiting { Error("wait") } else { panic("bad") }
let matched = match result { Ok(value) => value; Error(err) => panic(err); else => none }
let repeated = while ready { continue }
let forever = loop { break }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    let value = module.items[0].body.as_ref().ok_or("expected value body")?;
    assert!(matches!(value.kind, tune_hir::expr::ExprKind::Propagate(_)));

    let task = module.items[1].body.as_ref().ok_or("expected task body")?;
    assert!(matches!(task.kind, tune_hir::expr::ExprKind::Spawn(_)));

    let looped = module.items[2].body.as_ref().ok_or("expected loop body")?;
    let tune_hir::expr::ExprKind::For { pattern, .. } = &looped.kind else {
        return Err("expected for expression");
    };
    assert!(matches!(
        pattern.kind,
        tune_hir::pattern::PatternKind::Binding(ref name) if name == "item"
    ));

    let numbers = module.items[3]
        .body
        .as_ref()
        .ok_or("expected numbers body")?;
    let tune_hir::expr::ExprKind::Sequence(elements) = &numbers.kind else {
        return Err("expected sequence literal");
    };
    assert_eq!(elements.len(), 3);

    let callable = module.items[4]
        .body
        .as_ref()
        .ok_or("expected callable body")?;
    let tune_hir::expr::ExprKind::CallableValue { params, body } = &callable.kind else {
        return Err("expected callable value");
    };
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name.as_deref(), Some("x"));
    assert!(params[0].shape.is_some());
    assert!(matches!(body.kind, tune_hir::expr::ExprKind::Name(_)));

    let block = module.items[5].body.as_ref().ok_or("expected block body")?;
    let tune_hir::expr::ExprKind::Block(exprs) = &block.kind else {
        return Err("expected block expression");
    };
    assert!(matches!(
        exprs[0].kind,
        tune_hir::expr::ExprKind::Let { .. }
    ));
    assert!(matches!(
        exprs[1].kind,
        tune_hir::expr::ExprKind::Assign { .. }
    ));
    assert!(matches!(exprs[2].kind, tune_hir::expr::ExprKind::Return(_)));

    let grouped = module.items[6]
        .body
        .as_ref()
        .ok_or("expected grouped body")?;
    assert!(matches!(
        grouped.kind,
        tune_hir::expr::ExprKind::Binary {
            op: tune_hir::expr::BinaryOp::Add,
            ..
        }
    ));

    let ops = module.items[7].body.as_ref().ok_or("expected ops body")?;
    let tune_hir::expr::ExprKind::Binary { op, .. } = &ops.kind else {
        return Err("expected binary expression");
    };
    assert_eq!(*op, tune_hir::expr::BinaryOp::Or);

    let branched = module.items[8]
        .body
        .as_ref()
        .ok_or("expected branched body")?;
    let tune_hir::expr::ExprKind::If {
        branches,
        else_branch,
    } = &branched.kind
    else {
        return Err("expected if expression");
    };
    assert_eq!(branches.len(), 2);
    assert!(else_branch.is_some());

    let matched = module.items[9]
        .body
        .as_ref()
        .ok_or("expected matched body")?;
    let tune_hir::expr::ExprKind::Match { arms, .. } = &matched.kind else {
        return Err("expected match expression");
    };
    assert_eq!(arms.len(), 3);
    assert!(matches!(
        &arms[0].pattern.kind,
        tune_hir::pattern::PatternKind::Variant { name, args }
            if name == "Ok" && args.len() == 1
    ));
    assert!(matches!(
        &arms[1].pattern.kind,
        tune_hir::pattern::PatternKind::Variant { name, args }
            if name == "Error" && args.len() == 1
    ));
    assert!(matches!(
        arms[2].pattern.kind,
        tune_hir::pattern::PatternKind::Else
    ));

    let repeated = module.items[10]
        .body
        .as_ref()
        .ok_or("expected repeated body")?;
    assert!(matches!(
        repeated.kind,
        tune_hir::expr::ExprKind::While { .. }
    ));

    let forever = module.items[11]
        .body
        .as_ref()
        .ok_or("expected forever body")?;
    assert!(matches!(forever.kind, tune_hir::expr::ExprKind::Loop(_)));

    Ok(())
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
