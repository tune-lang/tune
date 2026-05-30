#[test]
fn numeric_literals_materialize_by_target_fit() {
    let twenty = tune_shape::LiteralFact::Numeric { text: "20".into() };
    let too_large_for_byte = tune_shape::LiteralFact::Numeric { text: "300".into() };
    let too_large_for_int = tune_shape::LiteralFact::Numeric {
        text: "9223372036854775808".into(),
    };
    let too_large_for_size = tune_shape::LiteralFact::Numeric {
        text: "18446744073709551616".into(),
    };
    let max_size = tune_shape::LiteralFact::Numeric {
        text: u64::MAX.to_string(),
    };

    assert!(tune_shape::can_materialize(
        &twenty,
        &tune_shape::Shape::Byte
    ));
    assert!(tune_shape::can_materialize(
        &twenty,
        &tune_shape::Shape::Int
    ));
    assert!(tune_shape::can_materialize(
        &twenty,
        &tune_shape::Shape::Size
    ));
    assert!(tune_shape::can_materialize(
        &max_size,
        &tune_shape::Shape::Size
    ));
    assert!(tune_shape::can_materialize(
        &twenty,
        &tune_shape::Shape::Float
    ));
    assert!(!tune_shape::can_materialize(
        &too_large_for_byte,
        &tune_shape::Shape::Byte
    ));
    assert!(!tune_shape::can_materialize(
        &too_large_for_int,
        &tune_shape::Shape::Int
    ));
    assert!(!tune_shape::can_materialize(
        &too_large_for_size,
        &tune_shape::Shape::Size
    ));
}

#[test]
fn float_literals_do_not_implicitly_round_to_integer_targets() {
    let float = tune_shape::LiteralFact::Numeric { text: "2.5".into() };

    assert!(!tune_shape::can_materialize(
        &float,
        &tune_shape::Shape::Byte
    ));
    assert!(!tune_shape::can_materialize(
        &float,
        &tune_shape::Shape::Int
    ));
    assert!(!tune_shape::can_materialize(
        &float,
        &tune_shape::Shape::Size
    ));
    assert!(tune_shape::can_materialize(
        &float,
        &tune_shape::Shape::Float
    ));
}

#[test]
fn sequence_literals_materialize_elementwise() {
    let sequence = tune_shape::LiteralFact::Sequence {
        elements: vec![
            tune_shape::LiteralFact::Numeric { text: "1".into() },
            tune_shape::LiteralFact::Numeric { text: "2".into() },
        ],
    };

    assert!(tune_shape::can_materialize(
        &sequence,
        &tune_shape::Shape::Sequence(Box::new(tune_shape::Shape::Int))
    ));
    assert!(!tune_shape::can_materialize(
        &sequence,
        &tune_shape::Shape::Sequence(Box::new(tune_shape::Shape::String))
    ));
}

#[test]
fn unrelated_literals_do_not_materialize() {
    let string = tune_shape::LiteralFact::String {
        segments: vec!["value".into()],
    };

    assert!(tune_shape::can_materialize(
        &string,
        &tune_shape::Shape::String
    ));
    assert!(!tune_shape::can_materialize(
        &string,
        &tune_shape::Shape::Int
    ));
}

#[test]
fn extracts_literal_facts_from_hir_bodies() -> Result<(), &'static str> {
    let source = r#"
let number = 20
let text = "hello"
let values = [1, 2, 3]
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    let number = module.items[0]
        .body
        .as_ref()
        .ok_or("expected number body")?;
    assert_eq!(
        tune_shape::expr_literal_fact(number),
        Some(tune_shape::LiteralFact::Numeric { text: "20".into() })
    );

    let text = module.items[1].body.as_ref().ok_or("expected text body")?;
    assert!(matches!(
        tune_shape::expr_literal_fact(text),
        Some(tune_shape::LiteralFact::String { .. })
    ));

    let values = module.items[2]
        .body
        .as_ref()
        .ok_or("expected values body")?;
    let Some(tune_shape::LiteralFact::Sequence { elements }) =
        tune_shape::expr_literal_fact(values)
    else {
        return Err("expected sequence literal fact");
    };
    assert_eq!(elements.len(), 3);
    assert!(tune_shape::can_materialize(
        &tune_shape::LiteralFact::Sequence { elements },
        &tune_shape::Shape::Sequence(Box::new(tune_shape::Shape::Int))
    ));

    Ok(())
}

#[test]
fn analysis_records_expression_materialization_target() -> Result<(), &'static str> {
    let source = "let result: Size = 3";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analyses = tune_shape::analyze_module(&module, &resolved);
    let body = module.items[0].body.as_ref().ok_or("expected body")?;

    assert!(analyses[0].diagnostics.is_empty());
    assert_eq!(
        analyses[0].materializations,
        vec![tune_shape::ExprMaterialization {
            expr: body.id,
            plan: tune_shape::MaterializationPlan {
                target: tune_shape::Shape::Size,
                commitment: tune_shape::Commitment::PerUse,
            },
            span: body.span,
        }]
    );

    Ok(())
}

#[test]
fn unmaterialized_numeric_binding_solves_from_later_rhs_shape() {
    let source = r#"
let make_float(): Float = 2.5
let result = {
  let x = 0
  x = make_float()
  x
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert!(analysis.frame.bindings.iter().any(|binding| {
        binding.name.as_deref() == Some("x") && binding.storage_shape == tune_shape::Shape::Float
    }));
}

#[test]
fn call_site_materialization_does_not_commit_literal_binding() -> Result<(), &'static str> {
    let source = r#"
let takes_byte(value: Byte): Byte = value
let takes_int(value: Int): Int = value
let result = {
  let x = 20
  let a: Byte = takes_byte(x)
  let b: Int = takes_int(x)
  b
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[2]);

    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    let x = analysis
        .frame
        .bindings
        .iter()
        .find(|binding| binding.name.as_deref() == Some("x"))
        .ok_or("x binding should be tracked")?;
    assert_eq!(x.storage_shape, tune_shape::Shape::Hole);
    assert!(matches!(
        x.literal_fact,
        Some(tune_shape::LiteralFact::Numeric { .. })
    ));
    Ok(())
}

#[test]
fn representation_specific_index_commits_literal_binding() {
    let source = r#"
let result = {
  let values = [1, 2]
  let index = 0
  let value: Int = values[index]
  index = 1.5
  value
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH),
        "{:?}",
        analysis.diagnostics
    );
    assert!(analysis.frame.bindings.iter().any(|binding| {
        binding.name.as_deref() == Some("index") && binding.storage_shape == tune_shape::Shape::Size
    }));
}
