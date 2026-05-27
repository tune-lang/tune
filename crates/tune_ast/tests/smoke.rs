#[test]
fn root_view_lists_top_level_items() -> Result<(), &'static str> {
    let source = r#"
-- docs
tag tool {}
pub let run(input) = input
struct Counter {}
enum Result {}
let value = 1
"#;
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root CST should cast to AST root")?;

    let items = root.items().collect::<Vec<_>>();

    assert_eq!(items.len(), 5);
    assert!(matches!(items[0], tune_ast::nodes::Item::Tag(_)));
    assert!(matches!(items[1], tune_ast::nodes::Item::Pub(_)));
    assert!(matches!(items[2], tune_ast::nodes::Item::Struct(_)));
    assert!(matches!(items[3], tune_ast::nodes::Item::Enum(_)));
    assert!(matches!(items[4], tune_ast::nodes::Item::Let(_)));

    Ok(())
}

#[test]
fn root_view_attaches_placement_docs_to_items() -> Result<(), &'static str> {
    let source = r#"
-- Tool tag docs.
tag tool {}

-/
Run docs line one.
Run docs line two.
/-
pub let run(input) = input
"#;
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let items = root.documented_items();

    assert_eq!(items.len(), 2);
    assert_eq!(items[0].doc_text(source).as_deref(), Some("Tool tag docs."));
    assert_eq!(
        items[1].doc_text(source).as_deref(),
        Some("Run docs line one.\nRun docs line two.")
    );

    Ok(())
}

#[test]
fn root_view_attaches_signature_placement_docs_to_callables() -> Result<(), &'static str> {
    let source = r#"
pub let run(input: String): String -/
Run docs below signature.
/- = input
"#;
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let items = root.documented_items();

    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].doc_text(source).as_deref(),
        Some("Run docs below signature.")
    );

    Ok(())
}

#[test]
fn root_view_attaches_tag_applications_to_items() -> Result<(), &'static str> {
    let source = r#"
tag tool {}

-- Search docs.
@tool
@route(path: "/search", capability = Capability.Read)
pub let search(query) = query
"#;
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let items = root.documented_items();

    assert_eq!(items.len(), 2);
    assert!(items[0].tags.is_empty());
    assert_eq!(items[1].doc_text(source).as_deref(), Some("Search docs."));
    assert_eq!(items[1].tags.len(), 2);
    assert_eq!(items[1].tags[0].name(source), Some("tool"));
    assert_eq!(items[1].tags[1].name(source), Some("route"));
    let args = items[1].tags[1].args();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0].name(source), Some("path"));
    assert!(args[0].value_expr().is_some());
    assert_eq!(args[1].name(source), Some("capability"));
    assert!(args[1].value_expr().is_some());

    Ok(())
}

#[test]
fn import_view_exposes_path() -> Result<(), &'static str> {
    let source = r#"import "std/json""#;
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let Some(tune_ast::nodes::Item::Import(import)) = root.items().next() else {
        return Err("expected import item");
    };

    assert_eq!(import.path(source), Some("std/json"));

    Ok(())
}

#[test]
fn declaration_views_expose_names_and_callable_form() -> Result<(), &'static str> {
    let callable_source = "let run(input) = input";
    let callable = tune_syntax::parse(callable_source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&callable.cst)
        .ok_or("callable root should cast")?;
    let Some(tune_ast::nodes::Item::Let(let_decl)) = root.items().next() else {
        return Err("callable declaration should be a let item");
    };

    assert!(let_decl.is_callable_decl());
    assert_eq!(let_decl.name(callable_source), Some("run"));

    let binding_source = "let value = 1";
    let binding = tune_syntax::parse(binding_source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&binding.cst)
        .ok_or("binding root should cast")?;
    let Some(tune_ast::nodes::Item::Let(let_decl)) = root.items().next() else {
        return Err("binding should be a let item");
    };

    assert!(!let_decl.is_callable_decl());
    assert_eq!(let_decl.name(binding_source), Some("value"));

    Ok(())
}

#[test]
fn callable_decl_exposes_params_and_return_shape() -> Result<(), &'static str> {
    let source = "let parse(text: String, strict: Bool): Result = text";
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let Some(tune_ast::nodes::Item::Let(let_decl)) = root.items().next() else {
        return Err("expected callable declaration");
    };
    let params = let_decl.params().ok_or("expected params")?;
    let params = params.params().collect::<Vec<_>>();

    assert_eq!(params.len(), 2);
    assert_eq!(params[0].name(source), Some("text"));
    assert!(params[0].shape_annotation().is_some());
    assert_eq!(params[1].name(source), Some("strict"));
    assert!(let_decl.shape_annotation().is_some());

    Ok(())
}

#[test]
fn declaration_views_expose_body_members() -> Result<(), &'static str> {
    let source = r#"
struct User {
  -- Name docs.
  name: String
  -- Age docs.
  age: Int
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
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let items = root.items().collect::<Vec<_>>();

    let tune_ast::nodes::Item::Struct(struct_decl) = items[0] else {
        return Err("expected struct");
    };
    let fields = struct_decl.fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].field.name(source), Some("name"));
    assert_eq!(fields[0].doc_text(source).as_deref(), Some("Name docs."));
    assert!(fields[0].field.shape_annotation().is_some());
    assert_eq!(fields[1].doc_text(source).as_deref(), Some("Age docs."));

    let tune_ast::nodes::Item::Enum(enum_decl) = items[1] else {
        return Err("expected enum");
    };
    let variants = enum_decl.variants();
    assert_eq!(variants.len(), 2);
    assert_eq!(variants[0].variant.name(source), Some("Ok"));
    assert_eq!(variants[0].variant.payload_shapes().len(), 1);

    let tune_ast::nodes::Item::Tag(tag_decl) = items[2] else {
        return Err("expected tag");
    };
    assert_eq!(tag_decl.fields().len(), 1);

    Ok(())
}

#[test]
fn declaration_names_only_use_direct_tokens() -> Result<(), &'static str> {
    let source = "struct { field: Int }";
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let Some(tune_ast::nodes::Item::Struct(struct_decl)) = root.items().next() else {
        return Err("expected recovered struct");
    };

    assert_eq!(struct_decl.name(source), None);
    assert_eq!(struct_decl.fields()[0].field.name(source), Some("field"));

    Ok(())
}

#[test]
fn let_decl_exposes_shape_annotation_view() -> Result<(), &'static str> {
    let source = "let value: [Int | String]? = none";
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let Some(tune_ast::nodes::Item::Let(let_decl)) = root.items().next() else {
        return Err("expected let item");
    };
    let shape = let_decl
        .shape_annotation()
        .ok_or("expected shape annotation")?;

    assert!(matches!(shape, tune_ast::nodes::Shape::Optional(_)));

    Ok(())
}

#[test]
fn let_decl_exposes_body_expression_view() -> Result<(), &'static str> {
    let source = "let value = spawn items[0].load()!\nlet numbers = [1, 2, 3]";
    let parsed = tune_syntax::parse(source);
    let root = <tune_ast::nodes::Root<'_> as tune_ast::AstNode<'_>>::cast(&parsed.cst)
        .ok_or("root should cast")?;
    let Some(tune_ast::nodes::Item::Let(let_decl)) = root.items().next() else {
        return Err("expected let item");
    };
    let body = let_decl.body_expr().ok_or("expected body expression")?;

    assert!(matches!(body, tune_ast::nodes::Expr::Spawn(_)));
    assert!(
        body.child_exprs()
            .iter()
            .any(|expr| matches!(expr, tune_ast::nodes::Expr::Propagate(_)))
    );

    let Some(tune_ast::nodes::Item::Let(numbers)) = root.items().nth(1) else {
        return Err("expected sequence literal item");
    };
    assert!(matches!(
        numbers.body_expr(),
        Some(tune_ast::nodes::Expr::Sequence(_))
    ));

    Ok(())
}
