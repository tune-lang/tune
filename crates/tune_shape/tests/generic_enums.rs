#[test]
fn generic_enum_constructors_solve_payload_type_params() -> Result<(), &'static str> {
    let source = r#"
enum Boxed<T> {
  Value(T)
  Pair(T, T)
}
let boxed: Boxed<String> = Value("hello")
let paired: Boxed<String | Bool> = Pair("hello", true)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let boxed_body = module.items[1].body.as_ref().ok_or("expected boxed body")?;
    let paired_body = module.items[2]
        .body
        .as_ref()
        .ok_or("expected paired body")?;

    assert_eq!(
        tune_shape::expr_shape_fact(boxed_body, &module, &resolved),
        Some(tune_shape::Shape::Apply {
            nominal: tune_shape::NominalShape::new(module.items[0].id, "Boxed"),
            args: vec![tune_shape::Shape::Literal(
                tune_shape::LiteralFact::String {
                    segments: vec!["hello".into()],
                }
            )],
        })
    );
    assert_eq!(
        tune_shape::expr_shape_fact(paired_body, &module, &resolved),
        Some(tune_shape::Shape::Apply {
            nominal: tune_shape::NominalShape::new(module.items[0].id, "Boxed"),
            args: vec![tune_shape::Shape::Union(vec![
                tune_shape::Shape::Literal(tune_shape::LiteralFact::String {
                    segments: vec!["hello".into()],
                }),
                tune_shape::Shape::Literal(tune_shape::LiteralFact::Bool(true)),
            ])],
        })
    );

    Ok(())
}

#[test]
fn generic_enum_constructors_solve_nested_generic_payloads() -> Result<(), &'static str> {
    let source = r#"
enum Wrapped<T> {
  Value(Inner<T>)
}
enum Inner<T> {
  InnerValue(T)
}
let wrapped: Wrapped<Int> = Value(InnerValue(1))
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let body = module.items[2].body.as_ref().ok_or("expected body")?;
    assert_eq!(
        tune_shape::expr_shape_fact(body, &module, &resolved),
        Some(tune_shape::Shape::Apply {
            nominal: tune_shape::NominalShape::new(module.items[0].id, "Wrapped"),
            args: vec![tune_shape::Shape::Literal(
                tune_shape::LiteralFact::Numeric { text: "1".into() }
            )],
        })
    );

    Ok(())
}
