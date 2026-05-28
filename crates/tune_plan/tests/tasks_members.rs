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
            .any(|op| matches!(op, tune_plan::PlanOp::TaskJoin { .. }))
    );
    assert!(!plan.ops.iter().any(|op| {
        matches!(
            op,
            tune_plan::PlanOp::FieldGet { field, .. } if field == "join"
        )
    }));

    Ok(())
}

#[test]
fn spawned_struct_construct_uses_shared_state_plan() -> Result<(), &'static str> {
    let source = r#"
struct Counter {
  value: Int
}
let run(): Task<Counter> = spawn Counter {
  value = 1
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let plan = tune_plan::lower_resolved_module_item_to_plan(&module, &module.items[1], &resolved)
        .ok_or("run plan should lower")?;

    assert!(plan.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::StructConstruct {
            escape: tune_plan::StructEscapeReason::SpawnBoundary,
            state: tune_plan::StructStatePlan::SHARED,
            ..
        }
    )));
    assert!(
        plan.ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::Spawn { .. }))
    );

    Ok(())
}

#[test]
fn module_context_resolves_member_ids_and_materialization_slots() -> Result<(), &'static str> {
    let source = r#"
struct Stack {
  value: Int
  get(index: Size): Int = index
  len(): Size = 0
  Stack[index: Size]: Int = index
  [items] = items
}
let stack: Stack = [1, 2]
let first(items: Stack) = items[0]
let named(items: Stack, value: Int) = { items.value = value; items.value }
let member(items: Stack) = items.get(0)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    assert!(resolved.diagnostics.is_empty());

    let stack = tune_plan::lower_resolved_module_item_to_plan(&module, &module.items[1], &resolved)
        .ok_or("stack plan should lower")?;
    assert!(
        stack
            .ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::Materialize { .. }))
    );

    let first = tune_plan::lower_resolved_module_item_to_plan(&module, &module.items[2], &resolved)
        .ok_or("first plan should lower")?;
    assert!(first.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::SequenceGet {
            index_member: Some(_),
            ..
        }
    )));

    let named = tune_plan::lower_resolved_module_item_to_plan(&module, &module.items[3], &resolved)
        .ok_or("named plan should lower")?;
    assert!(named.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::FieldSet {
            member: Some(_),
            field,
            ..
        } if field == "value"
    )));
    assert!(named.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::FieldGet {
            member: Some(_),
            field,
            ..
        } if field == "value"
    )));

    let member =
        tune_plan::lower_resolved_module_item_to_plan(&module, &module.items[4], &resolved)
            .ok_or("member plan should lower")?;
    assert!(member.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::MemberCall {
            member: Some(_),
            name,
            ..
        } if name == "get"
    )));

    Ok(())
}
