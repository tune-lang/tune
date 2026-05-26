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
fn root_view_attaches_tag_applications_to_items() -> Result<(), &'static str> {
    let source = r#"
tag tool {}

-- Search docs.
@tool
@route(path: "/search")
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
