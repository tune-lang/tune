#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn lowers_hir_body_to_semantic_plan_ops() -> Result<(), &'static str> {
    let source = r#"
let handle(item) = item
let run(items) = spawn items[0].load()!
let each(items) = for item in items { handle(item) }
let values = [1, 2]
let scoped(input) = { let f = _(x) = x; input = f(input); return input }
let mutate(user, values) = { user.name = "Rey"; values[0] = user.name }
let ops(value, other) = not value and other is not none
let branch(value, ready, waiting) = if ready { value } elif waiting { panic("wait") } else { panic("bad") }
let select(result, value) = match result { value => value; else => panic("bad") }
let repeated(ready) = while ready { continue }
let forever() = loop { break }
let ok(value) = Ok(value)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let run = tune_plan::lower_resolved_item_to_plan(&module.items[1], &resolved)
        .ok_or("expected run plan")?;
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
    assert!(
        run.ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::ResultPropagate { span: Some(_), .. }))
    );
    assert!(run.ops.contains(&tune_plan::PlanOp::Spawn));

    let each = tune_plan::lower_resolved_item_to_plan(&module.items[2], &resolved)
        .ok_or("expected each plan")?;
    assert!(each.ops.contains(&tune_plan::PlanOp::FiniteFor));
    assert!(each.ops.contains(&tune_plan::PlanOp::DirectCall {
        target: tune_hir::HirId(0)
    }));

    let values = tune_plan::lower_resolved_item_to_plan(&module.items[3], &resolved)
        .ok_or("expected values plan")?;
    assert_eq!(
        values
            .ops
            .iter()
            .filter(|op| **op == tune_plan::PlanOp::SequencePush)
            .count(),
        2
    );

    let scoped = tune_plan::lower_resolved_item_to_plan(&module.items[4], &resolved)
        .ok_or("expected scoped plan")?;
    assert!(scoped.ops.contains(&tune_plan::PlanOp::CallableValue));
    assert!(
        scoped
            .ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::LocalLet { local: Some(_) }))
    );
    assert!(
        scoped
            .ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::BindingSet { target: Some(_) }))
    );
    assert!(scoped.ops.contains(&tune_plan::PlanOp::Return));

    let mutate = tune_plan::lower_resolved_item_to_plan(&module.items[5], &resolved)
        .ok_or("expected mutate plan")?;
    assert!(mutate.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::FieldSet { field } if field == "name"
    )));
    assert!(
        mutate
            .ops
            .contains(&tune_plan::PlanOp::SequenceSet { checked: true })
    );
    assert!(!mutate.ops.contains(&tune_plan::PlanOp::Assign));

    let ops = tune_plan::lower_resolved_item_to_plan(&module.items[6], &resolved)
        .ok_or("expected ops plan")?;
    assert!(ops.ops.contains(&tune_plan::PlanOp::UnaryOp {
        op: tune_hir::expr::UnaryOp::Not
    }));
    assert!(ops.ops.contains(&tune_plan::PlanOp::BinaryOp {
        op: tune_hir::expr::BinaryOp::IsNot
    }));

    let branch = tune_plan::lower_resolved_item_to_plan(&module.items[7], &resolved)
        .ok_or("expected branch plan")?;
    assert!(branch.ops.contains(&tune_plan::PlanOp::If));
    assert!(branch.ops.contains(&tune_plan::PlanOp::Panic));

    let select = tune_plan::lower_resolved_item_to_plan(&module.items[8], &resolved)
        .ok_or("expected select plan")?;
    assert!(select.ops.contains(&tune_plan::PlanOp::Match));

    let repeated = tune_plan::lower_resolved_item_to_plan(&module.items[9], &resolved)
        .ok_or("expected repeated plan")?;
    assert!(repeated.ops.contains(&tune_plan::PlanOp::While));
    assert!(repeated.ops.contains(&tune_plan::PlanOp::Continue));

    let forever = tune_plan::lower_resolved_item_to_plan(&module.items[10], &resolved)
        .ok_or("expected forever plan")?;
    assert!(forever.ops.contains(&tune_plan::PlanOp::Loop));
    assert!(forever.ops.contains(&tune_plan::PlanOp::Break));

    let ok = tune_plan::lower_resolved_item_to_plan(&module.items[11], &resolved)
        .ok_or("expected ok plan")?;
    assert!(ok.ops.contains(&tune_plan::PlanOp::VariantConstruct {
        variant: tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Ok)
    }));

    Ok(())
}
