#[test]
fn numeric_literals_materialize_by_target_fit() {
    let twenty = tune_shape::LiteralFact::Numeric { text: "20".into() };
    let too_large_for_byte = tune_shape::LiteralFact::Numeric { text: "300".into() };

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
        &twenty,
        &tune_shape::Shape::Float
    ));
    assert!(!tune_shape::can_materialize(
        &too_large_for_byte,
        &tune_shape::Shape::Byte
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
