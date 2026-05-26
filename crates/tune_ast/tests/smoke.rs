#[test]
fn root_view_lists_top_level_items() -> Result<(), &'static str> {
    let source = r#"
--- docs
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
