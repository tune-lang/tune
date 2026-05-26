#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn lowers_hir_body_to_semantic_plan_ops() -> Result<(), &'static str> {
    let source = r#"
let run(items) = spawn items[0].load()!
let each(items) = for item in items { handle(item) }
let values = [1, 2]
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    let run = tune_plan::lower_item_to_plan(&module.items[0]).ok_or("expected run plan")?;
    assert_eq!(run.name, "run");
    assert!(
        run.ops
            .contains(&tune_plan::PlanOp::SequenceGet { checked: true })
    );
    assert!(run.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::FieldGet { field } if field == "load"
    )));
    assert!(run.ops.contains(&tune_plan::PlanOp::BoundCall));
    assert!(run.ops.contains(&tune_plan::PlanOp::ResultPropagate));
    assert!(run.ops.contains(&tune_plan::PlanOp::Spawn));

    let each = tune_plan::lower_item_to_plan(&module.items[1]).ok_or("expected each plan")?;
    assert!(each.ops.contains(&tune_plan::PlanOp::FiniteFor));

    let values = tune_plan::lower_item_to_plan(&module.items[2]).ok_or("expected values plan")?;
    assert_eq!(
        values
            .ops
            .iter()
            .filter(|op| **op == tune_plan::PlanOp::SequencePush)
            .count(),
        2
    );

    Ok(())
}
