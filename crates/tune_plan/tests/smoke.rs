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
    assert!(
        run.ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::Spawn { span: Some(_), .. }))
    );

    let each = tune_plan::lower_resolved_item_to_plan(&module.items[2], &resolved)
        .ok_or("expected each plan")?;
    assert!(each.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::FiniteFor {
            contract: tune_plan::FiniteForContract {
                source_evaluated_once: true,
                length_evaluated_once: true,
                ..
            },
            span: Some(_),
            ..
        }
    )));
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
    assert!(branch.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::If {
            branches,
            else_body: Some(_),
            span: Some(_)
        } if branches.len() == 2
    )));
    assert!(branch.ops.contains(&tune_plan::PlanOp::Panic));

    let select = tune_plan::lower_resolved_item_to_plan(&module.items[8], &resolved)
        .ok_or("expected select plan")?;
    assert!(select.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::Match {
            arms,
            span: Some(_),
            ..
        } if arms.len() == 2
    )));

    let repeated = tune_plan::lower_resolved_item_to_plan(&module.items[9], &resolved)
        .ok_or("expected repeated plan")?;
    assert!(
        repeated
            .ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::While { span: Some(_), .. }))
    );
    assert!(repeated.ops.contains(&tune_plan::PlanOp::Continue));

    let forever = tune_plan::lower_resolved_item_to_plan(&module.items[10], &resolved)
        .ok_or("expected forever plan")?;
    assert!(
        forever
            .ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::Loop { span: Some(_), .. }))
    );
    assert!(forever.ops.contains(&tune_plan::PlanOp::Break));

    let ok = tune_plan::lower_resolved_item_to_plan(&module.items[11], &resolved)
        .ok_or("expected ok plan")?;
    assert!(ok.ops.contains(&tune_plan::PlanOp::VariantConstruct {
        variant: tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Ok)
    }));

    Ok(())
}

#[test]
fn semantic_plan_has_typed_materialization_and_meta_slots() {
    let materialize = tune_plan::PlanOp::Materialize {
        plan: tune_shape::MaterializationPlan {
            target: tune_shape::Shape::Sequence(Box::new(tune_shape::Shape::Int)),
            commitment: tune_shape::Commitment::PerUse,
        },
    };
    assert!(matches!(
        materialize,
        tune_plan::PlanOp::Materialize {
            plan: tune_shape::MaterializationPlan {
                commitment: tune_shape::Commitment::PerUse,
                ..
            }
        }
    ));

    let meta = tune_plan::PlanOp::Meta {
        plan: tune_plan::meta::MetaPlan::CompilerFact {
            owner: tune_resolve::FactOwner::Item(tune_hir::HirId(0)),
            kind: tune_resolve::CompilerFactKind::Doc,
        },
    };
    assert!(matches!(
        meta,
        tune_plan::PlanOp::Meta {
            plan: tune_plan::meta::MetaPlan::CompilerFact {
                kind: tune_resolve::CompilerFactKind::Doc,
                ..
            }
        }
    ));
}

#[test]
fn task_join_lowers_to_dedicated_plan_op() -> Result<(), &'static str> {
    let source = "let wait(task) = task.join()";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    assert!(resolved.diagnostics.is_empty());

    let plan = tune_plan::lower_resolved_item_to_plan(&module.items[0], &resolved)
        .ok_or("function body should lower")?;

    assert!(
        plan.ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::TaskJoin))
    );
    assert!(!plan.ops.iter().any(|op| {
        matches!(
            op,
            tune_plan::PlanOp::FieldGet { field } if field == "join"
        )
    }));

    Ok(())
}
