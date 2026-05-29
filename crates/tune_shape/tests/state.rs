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

#[test]
fn state_frame_joins_current_shapes_without_changing_storage() {
    let key = tune_shape::BindingKey::Local(tune_resolve::LocalId(3));
    let mut left = tune_shape::StateFrame::new();
    let mut right = tune_shape::StateFrame::new();

    assert!(left.define(tune_shape::BindingState::new(
        key,
        Some("value".into()),
        tune_shape::Shape::Hole,
        tune_shape::Shape::Int,
        None,
    )));
    assert!(right.define(tune_shape::BindingState::new(
        key,
        Some("value".into()),
        tune_shape::Shape::Hole,
        tune_shape::Shape::String,
        None,
    )));

    assert_eq!(left.join_from(&right), Ok(()));
    assert_eq!(
        left.get(key).map(|binding| &binding.storage_shape),
        Some(&tune_shape::Shape::Hole)
    );
    assert_eq!(
        left.get(key).map(|binding| &binding.current_shape),
        Some(&tune_shape::Shape::Union(vec![
            tune_shape::Shape::Int,
            tune_shape::Shape::String,
        ]))
    );
}

#[test]
fn state_frame_rejects_storage_shape_mismatch_on_join() {
    let key = tune_shape::BindingKey::Local(tune_resolve::LocalId(4));
    let mut left = tune_shape::StateFrame::new();
    let mut right = tune_shape::StateFrame::new();

    assert!(left.define(tune_shape::BindingState::new(
        key,
        Some("value".into()),
        tune_shape::Shape::Int,
        tune_shape::Shape::Int,
        None,
    )));
    assert!(right.define(tune_shape::BindingState::new(
        key,
        Some("value".into()),
        tune_shape::Shape::String,
        tune_shape::Shape::String,
        None,
    )));

    assert_eq!(
        left.join_from(&right),
        Err(tune_shape::StateJoinError::StorageMismatch(key))
    );
}

#[test]
fn analyzer_checks_assignment_storage_shapes() -> Result<(), &'static str> {
    let source = r#"
let run(value: Int) = {
  let local: Int = 1
  local = "bad"
  value
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.assignments.iter().any(|assignment| {
        assignment.expected == tune_shape::Shape::Int
            && matches!(assignment.actual, tune_shape::Shape::Literal(_))
    }));
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
    }));

    Ok(())
}

#[test]
fn analyzer_validates_finite_for_contracts_and_materializers() -> Result<(), &'static str> {
    let source = r#"
struct Stack {
  len(): Size = 0
  Stack[index: Size]: Int = index
  [items] = items
}
struct Bag {}
let stack: Stack = [1, 2]
let bag: Bag = [1, 2]
let each(items: Stack) = for item in items { item }
let ranged() = for item in 0..=2 { item }
let broken(value: Int) = for item in value { item }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let stack = tune_shape::analyze_item(&module, &resolved, &module.items[2]);
    assert!(
        stack
            .materializers
            .iter()
            .any(|materializer| materializer.materializer.is_some())
    );

    let bag = tune_shape::analyze_item(&module, &resolved, &module.items[3]);
    assert!(
        bag.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::MATERIALIZATION_FAILED
        })
    );

    let each = tune_shape::analyze_item(&module, &resolved, &module.items[4]);
    assert!(each.finite_for.iter().any(|finite| finite.contract
        == tune_shape::FiniteForContractKind::MemberAccess
        && finite.len_member.is_some()
        && finite.index_member.is_some()));

    let ranged = tune_shape::analyze_item(&module, &resolved, &module.items[5]);
    assert!(ranged.finite_for.iter().any(|finite| {
        finite.contract == tune_shape::FiniteForContractKind::Range
            && finite.len_member.is_none()
            && finite.index_member.is_none()
            && finite.iterable.0 != u64::MAX
    }));
    assert!(ranged.diagnostics.is_empty());

    let broken = tune_shape::analyze_item(&module, &resolved, &module.items[6]);
    assert!(
        broken.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::ITERATION_LEN_MISSING
        })
    );
    assert!(
        broken.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == tune_diagnostics::codes::ITERATION_INDEX_MISSING
        })
    );

    Ok(())
}

#[test]
fn analyzer_reports_non_exhaustive_enum_matches() -> Result<(), &'static str> {
    let source = r#"
enum Color {
  Red
  Blue
}
let choose(color: Color) = match color { Red => 1 }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == tune_diagnostics::codes::MATCH_NOT_EXHAUSTIVE })
    );

    Ok(())
}

#[test]
fn analyzer_joins_branch_state_after_if() -> Result<(), &'static str> {
    let source = r#"
let run(flag) = {
  let value: Int | String = 1
  if flag { value = 2 } else { value = "two" }
  value
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    let value = analysis
        .frame
        .bindings
        .iter()
        .find(|binding| binding.name.as_deref() == Some("value"))
        .ok_or("value binding should survive branch join")?;
    assert!(matches!(value.current_shape, tune_shape::Shape::Union(_)));

    Ok(())
}

#[test]
fn analyzer_warns_when_finite_for_source_is_mutated() -> Result<(), &'static str> {
    let source = r#"
struct Stack {
  len(): Size = 0
  Stack[index: Size]: Int = index
}
let each(items: Stack) = for item in items { items = items }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::ITERATION_SOURCE_MUTATED
    }));

    Ok(())
}

#[test]
fn analyzer_rejects_effectful_sequence_materializers() -> Result<(), &'static str> {
    let source = r#"
struct Stack {
  [items] = { items = []; items }
}
let stack: Stack = [1, 2]
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::MATERIALIZATION_FAILED
            && diagnostic.title == "sequence materializer is not pure"
    }));

    Ok(())
}

#[test]
fn analyzer_enforces_result_propagation_return_shape() -> Result<(), &'static str> {
    let source = r#"
let run(): Int = Error("bad")!
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::RESULT_PROPAGATION_ERROR
    }));

    Ok(())
}

#[test]
fn analyzer_warns_for_public_api_inference() -> Result<(), &'static str> {
    let source = "pub let run(input): Int = input";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    let diagnostic = analysis
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == tune_diagnostics::codes::PUBLIC_API_INFERENCE)
        .ok_or("public inference warning should be emitted")?;
    assert_eq!(
        diagnostic.title,
        "public callable has inferred signature shape"
    );
    assert!(diagnostic.facts.iter().any(|fact| {
        fact.entries
            .iter()
            .any(|entry| entry.message == "parameter `input` shape is inferred")
    }));

    Ok(())
}

#[test]
fn analyzer_warns_for_public_value_storage_inference() -> Result<(), &'static str> {
    let source = "pub let value = 1";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::PUBLIC_API_INFERENCE
            && diagnostic.title == "public value has inferred storage shape"
    }));

    Ok(())
}

#[test]
fn analyzer_commits_unannotated_literal_binding_storage() -> Result<(), &'static str> {
    let source = r#"
let result = {
  let x = 0
  x = "hello"
  x
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
            && diagnostic.title == "assigned value does not match storage shape"
    }));

    Ok(())
}

#[test]
fn analyzer_rejects_non_integer_executable_comparisons() -> Result<(), &'static str> {
    let source = r#"let result: Bool = "a" < 3"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::SHAPE_MISMATCH
            && diagnostic.title == "operator operands do not match executable integer operation"
    }));

    Ok(())
}
