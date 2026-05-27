#[test]
fn extracts_result_shapes_from_prelude_variant_constructors() -> Result<(), &'static str> {
    let source = r#"
let ok(value) = Ok(value)
let error(err) = Error(err)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let ok_body = module.items[0].body.as_ref().ok_or("expected ok body")?;
    let error_body = module.items[1].body.as_ref().ok_or("expected error body")?;

    assert!(matches!(
        tune_shape::expr_shape_fact(ok_body, &module, &resolved),
        Some(tune_shape::Shape::Result { ok, err })
            if *ok == tune_shape::Shape::Hole && *err == tune_shape::Shape::Hole
    ));
    assert!(matches!(
        tune_shape::expr_shape_fact(error_body, &module, &resolved),
        Some(tune_shape::Shape::Result { ok, err })
            if *ok == tune_shape::Shape::Hole && *err == tune_shape::Shape::Hole
    ));

    Ok(())
}

#[test]
fn propagation_shape_uses_result_ok_shape() -> Result<(), &'static str> {
    let source = "let value = Ok(1)!";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let body = module.items[0].body.as_ref().ok_or("expected body")?;

    assert_eq!(
        tune_shape::expr_shape_fact(body, &module, &resolved),
        Some(tune_shape::Shape::Literal(
            tune_shape::LiteralFact::Numeric { text: "1".into() }
        ))
    );

    Ok(())
}

#[test]
fn result_constructor_facts_union_variant_payloads_from_value_flow() -> Result<(), &'static str> {
    let source = r#"
let choose(ready, waiting, value) = if ready { Ok(value) } elif waiting { Error("wait") } else { Error(1) }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let body = module.items[0].body.as_ref().ok_or("expected body")?;

    assert_eq!(
        tune_shape::expr_result_constructor_shape_fact(body, &module, &resolved),
        Some(tune_shape::Shape::Result {
            ok: Box::new(tune_shape::Shape::Hole),
            err: Box::new(tune_shape::Shape::Union(vec![
                tune_shape::Shape::Literal(tune_shape::LiteralFact::String {
                    segments: vec!["wait".into()],
                }),
                tune_shape::Shape::Literal(tune_shape::LiteralFact::Numeric { text: "1".into() }),
            ])),
        })
    );

    Ok(())
}

#[test]
fn result_constructor_facts_include_explicit_returns() -> Result<(), &'static str> {
    let source = r#"
let choose(ready) = {
  if ready { return Ok(1) }
  return Error("bad")
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let body = module.items[0].body.as_ref().ok_or("expected body")?;

    assert_eq!(
        tune_shape::expr_result_constructor_shape_fact(body, &module, &resolved),
        Some(tune_shape::Shape::Result {
            ok: Box::new(tune_shape::Shape::Literal(
                tune_shape::LiteralFact::Numeric { text: "1".into() }
            )),
            err: Box::new(tune_shape::Shape::Literal(
                tune_shape::LiteralFact::String {
                    segments: vec!["bad".into()],
                }
            )),
        })
    );

    Ok(())
}

#[test]
fn propagated_error_facts_union_only_bang_sites() -> Result<(), &'static str> {
    let source = r#"
let load = {
  Error("fs")!
  Error(1)!
  Ok("done")
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let body = module.items[0].body.as_ref().ok_or("expected body")?;

    assert_eq!(
        tune_shape::expr_propagated_error_shape_fact(body, &module, &resolved),
        Some(tune_shape::Shape::Union(vec![
            tune_shape::Shape::Literal(tune_shape::LiteralFact::String {
                segments: vec!["fs".into()],
            }),
            tune_shape::Shape::Literal(tune_shape::LiteralFact::Numeric { text: "1".into() }),
        ]))
    );

    Ok(())
}

#[test]
fn extracts_enum_shapes_from_variant_constructors() -> Result<(), &'static str> {
    let source = r#"
enum Color {
  Red
  Rgb(Int, Int, Int)
}
let red = Red
let rgb: Color = Rgb(1, 2, 3)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let red_body = module.items[1].body.as_ref().ok_or("expected red body")?;
    let rgb_body = module.items[2].body.as_ref().ok_or("expected rgb body")?;

    assert_eq!(
        tune_shape::expr_shape_fact(red_body, &module, &resolved),
        None
    );
    assert_eq!(
        tune_shape::expr_shape_fact(rgb_body, &module, &resolved),
        Some(tune_shape::Shape::Enum("Color".to_owned()))
    );

    Ok(())
}

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
            name: "Boxed".to_owned(),
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
            name: "Boxed".to_owned(),
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
fn shape_store_keeps_stable_ids_and_origins() -> Result<(), &'static str> {
    let mut store = tune_shape::ShapeStore::new();
    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(4),
        tune_diagnostics::ByteOffset::new(7),
    );

    let int_id = store
        .intern(tune_shape::Shape::Int, tune_shape::ShapeOrigin::Builtin)
        .ok_or("builtin shape should allocate")?;
    let annotation_id = store
        .intern(
            tune_shape::Shape::String,
            tune_shape::ShapeOrigin::Annotation(span),
        )
        .ok_or("annotation shape should allocate")?;

    assert_eq!(int_id, tune_shape::ShapeId(0));
    assert_eq!(annotation_id, tune_shape::ShapeId(1));
    assert_eq!(
        store.get(int_id).map(|fact| &fact.shape),
        Some(&tune_shape::Shape::Int)
    );
    assert_eq!(
        store.get(annotation_id).map(|fact| fact.origin),
        Some(tune_shape::ShapeOrigin::Annotation(span))
    );

    Ok(())
}

#[test]
fn interns_hir_shape_annotations_with_provenance() -> Result<(), &'static str> {
    let source = "let value: [Int | String]? = none";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let hir_shape = module.items[0]
        .shape
        .as_ref()
        .ok_or("expected HIR shape annotation")?;
    let mut store = tune_shape::ShapeStore::new();

    let id = tune_shape::intern_hir_shape(&mut store, hir_shape)
        .ok_or("shape store should allocate HIR shape")?;
    let fact = store.get(id).ok_or("shape fact should be retrievable")?;

    assert!(matches!(fact.shape, tune_shape::Shape::Optional(_)));
    assert!(matches!(
        fact.origin,
        tune_shape::ShapeOrigin::Annotation(_)
    ));

    Ok(())
}

#[test]
fn unresolved_hir_shape_lowering_stays_holey_without_resolution() -> Result<(), &'static str> {
    let source = "let value: Missing = none";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let hir_shape = module.items[0]
        .shape
        .as_ref()
        .ok_or("expected shape annotation")?;

    assert_eq!(
        tune_shape::lower_hir_shape(hir_shape),
        tune_shape::Shape::Hole
    );

    Ok(())
}

#[test]
fn resolved_hir_shape_reports_unknown_names() -> Result<(), &'static str> {
    let source = "let value: Missing = none";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let hir_shape = module.items[0]
        .shape
        .as_ref()
        .ok_or("expected HIR shape annotation")?;
    let mut store = tune_shape::ShapeStore::new();

    let (id, diagnostics) =
        tune_shape::intern_resolved_hir_shape(&mut store, hir_shape, &resolved.scope);

    assert!(id.is_some());
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        tune_diagnostics::codes::UNRESOLVED_NAME
    );

    Ok(())
}

#[test]
fn resolved_hir_shape_uses_declared_structs() -> Result<(), &'static str> {
    let source = "struct User {}\nlet value: User = none";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let hir_shape = module.items[1]
        .shape
        .as_ref()
        .ok_or("expected HIR shape annotation")?;

    let lowered = tune_shape::lower_resolved_hir_shape(hir_shape, &resolved.scope);

    assert!(lowered.diagnostics.is_empty());
    assert_eq!(lowered.shape, tune_shape::Shape::Struct("User".to_owned()));

    Ok(())
}

#[test]
fn resolved_hir_shape_lowers_result_and_task_generics() -> Result<(), &'static str> {
    let source = r#"
struct Config {}
enum ParseError {}
let parse(text: String): Result<Config, ParseError> = text
let background(): Task<Result<Config, ParseError>> = parse("")
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let result_shape = module.items[2]
        .shape
        .as_ref()
        .ok_or("expected result shape")?;
    let task_shape = module.items[3]
        .shape
        .as_ref()
        .ok_or("expected task shape")?;

    let lowered_result = tune_shape::lower_resolved_hir_shape(result_shape, &resolved.scope);
    let lowered_task = tune_shape::lower_resolved_hir_shape(task_shape, &resolved.scope);

    assert!(lowered_result.diagnostics.is_empty());
    assert!(lowered_task.diagnostics.is_empty());
    assert!(matches!(
        lowered_result.shape,
        tune_shape::Shape::Result { .. }
    ));
    assert!(matches!(lowered_task.shape, tune_shape::Shape::Task(_)));

    Ok(())
}
