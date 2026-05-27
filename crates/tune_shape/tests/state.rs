#[test]
fn binding_state_preserves_storage_current_and_literal_meaning() {
    let key = tune_shape::BindingKey::Local(tune_resolve::LocalId(0));
    let mut binding = tune_shape::BindingState::literal(
        key,
        Some("x".into()),
        tune_shape::Shape::Hole,
        tune_shape::LiteralFact::Numeric { text: "20".into() },
        None,
    );

    assert_eq!(binding.storage_shape, tune_shape::Shape::Hole);
    assert_eq!(
        binding.current_shape,
        tune_shape::Shape::Literal(tune_shape::LiteralFact::Numeric { text: "20".into() })
    );
    assert_eq!(
        binding.literal_fact,
        Some(tune_shape::LiteralFact::Numeric { text: "20".into() })
    );

    assert!(binding.commit_materialization(tune_shape::Shape::Byte));
    assert_eq!(binding.storage_shape, tune_shape::Shape::Hole);
    assert_eq!(binding.current_shape, tune_shape::Shape::Byte);
    assert_eq!(
        binding.materialization,
        Some(tune_shape::MaterializationPlan {
            target: tune_shape::Shape::Byte,
            commitment: tune_shape::Commitment::CommitBinding,
        })
    );
}

#[test]
fn binding_state_rejects_incompatible_literal_materialization() {
    let key = tune_shape::BindingKey::Local(tune_resolve::LocalId(1));
    let mut binding = tune_shape::BindingState::literal(
        key,
        Some("value".into()),
        tune_shape::Shape::Hole,
        tune_shape::LiteralFact::String {
            segments: vec!["hello".into()],
        },
        None,
    );

    assert!(!binding.commit_materialization(tune_shape::Shape::Int));
    assert!(matches!(
        binding.current_shape,
        tune_shape::Shape::Literal(tune_shape::LiteralFact::String { .. })
    ));
    assert!(binding.materialization.is_none());
}

#[test]
fn state_frame_tracks_bindings_by_typed_key() {
    let key = tune_shape::BindingKey::Local(tune_resolve::LocalId(2));
    let mut frame = tune_shape::StateFrame::new();

    assert!(frame.define(tune_shape::BindingState::new(
        key,
        Some("item".into()),
        tune_shape::Shape::Int,
        tune_shape::Shape::Int,
        None,
    )));
    assert!(!frame.define(tune_shape::BindingState::new(
        key,
        Some("duplicate".into()),
        tune_shape::Shape::String,
        tune_shape::Shape::String,
        None,
    )));

    assert!(frame.assign_literal(key, tune_shape::LiteralFact::Numeric { text: "255".into() }));
    assert!(frame.commit_materialization(key, tune_shape::Shape::Byte));
    assert_eq!(
        frame.get(key).map(|binding| &binding.current_shape),
        Some(&tune_shape::Shape::Byte)
    );
}
