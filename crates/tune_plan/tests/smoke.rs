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
let select(result, value) = match result { value => value; else panic("bad") }
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
    assert!(run.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::Spawn {
            body_ops,
            span: Some(_),
            ..
        } if body_ops.contains(&tune_plan::PlanOp::SequenceGet {
            checked: true,
            index_member: None
        }) && body_ops.iter().any(|op| matches!(
            op,
            tune_plan::PlanOp::MemberCall { name, .. } if name == "load"
        )) && body_ops.iter().any(|op| matches!(
            op,
            tune_plan::PlanOp::ResultPropagate { span: Some(_), .. }
        ))
    )));

    let each = tune_plan::lower_resolved_item_to_plan(&module.items[2], &resolved)
        .ok_or("expected each plan")?;
    assert!(each.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::FiniteFor {
            body_ops,
            contract: tune_plan::FiniteForContract {
                source_evaluated_once: true,
                length_evaluated_once: true,
                ..
            },
            span: Some(_),
            ..
        } if body_ops.iter().any(|body_op| matches!(
            body_op,
            tune_plan::PlanOp::DirectCall {
                target: tune_hir::HirId(0),
                arg_count: 1,
                span: Some(_),
            }
        ))
    )));

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
    assert!(
        scoped
            .ops
            .iter()
            .any(|op| matches!(op, tune_plan::PlanOp::CallableValue { .. }))
    );
    assert!(scoped.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::LocalLet {
            local: Some(_),
            initialized: true
        }
    )));
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
        tune_plan::PlanOp::FieldSet { field, .. } if field == "name"
    )));
    assert!(mutate.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::SequenceSet {
            checked: true,
            index_member: None,
            base: Some(_)
        }
    )));
    assert!(!mutate.ops.contains(&tune_plan::PlanOp::Assign));

    let ops = tune_plan::lower_resolved_item_to_plan(&module.items[6], &resolved)
        .ok_or("expected ops plan")?;
    assert!(plan_ops_contain_bool_and(&ops.ops));

    let branch = tune_plan::lower_resolved_item_to_plan(&module.items[7], &resolved)
        .ok_or("expected branch plan")?;
    assert!(branch.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::If {
            branches,
            else_body: Some(_),
            span: Some(_),
            ..
        } if branches.len() == 2
    )));
    assert!(branch.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::If { branches, .. }
            if branches
                .iter()
                .any(|branch| branch.body_ops.iter().any(|op| matches!(
                    op,
                    tune_plan::PlanOp::Panic { .. }
                )))
    )));

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
    assert!(repeated.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::While {
            body_ops,
            span: Some(_),
            ..
        } if body_ops.contains(&tune_plan::PlanOp::Continue)
    )));

    let forever = tune_plan::lower_resolved_item_to_plan(&module.items[10], &resolved)
        .ok_or("expected forever plan")?;
    assert!(forever.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::Loop {
            body_ops,
            span: Some(_),
            ..
        } if body_ops.contains(&tune_plan::PlanOp::Break)
    )));

    let ok = tune_plan::lower_resolved_item_to_plan(&module.items[11], &resolved)
        .ok_or("expected ok plan")?;
    assert!(ok.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::VariantConstruct {
            variant: tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Ok),
            arg_count: 1,
            span: Some(_),
        }
    )));

    Ok(())
}

fn plan_ops_contain_bool_and(ops: &[tune_plan::PlanOp]) -> bool {
    ops.iter().any(|op| match op {
        tune_plan::PlanOp::BoolAnd {
            lhs_ops,
            rhs_ops,
            span: Some(_),
        } => {
            lhs_ops.contains(&tune_plan::PlanOp::UnaryOp {
                op: tune_hir::expr::UnaryOp::Not,
            }) || plan_ops_contain_bool_and(rhs_ops)
        }
        tune_plan::PlanOp::BinaryOp {
            op: tune_hir::expr::BinaryOp::NotEqual,
            ..
        } => false,
        _ => false,
    })
}

#[test]
fn explicit_return_body_does_not_get_extra_implicit_return() -> Result<(), &'static str> {
    let source = "let main(): Int = return 1";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let plan = tune_plan::lower_resolved_item_to_plan(&module.items[0], &resolved)
        .ok_or("expected main plan")?;

    assert_eq!(
        plan.ops
            .iter()
            .filter(|op| **op == tune_plan::PlanOp::Return)
            .count(),
        1
    );

    Ok(())
}

#[test]
fn structural_match_lowers_to_known_member_witness() -> Result<(), &'static str> {
    let source = r#"
struct Duck {
  quack(): Int = 7
}
let duck: Duck = Duck {}
let result: Int = match duck {
  { quack(): Int } => quack()
  else 0
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let plan = tune_plan::lower_resolved_module_to_plan(&module, &resolved);
    let entry = plan.entry.ok_or("entry should lower")?;

    assert!(!plan_ops_contain_match(&entry.ops));
    assert!(entry.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::MemberCall {
            member: Some(_),
            name,
            arg_count: 0,
            ..
        } if name == "quack"
    )));

    Ok(())
}

fn plan_ops_contain_match(ops: &[tune_plan::PlanOp]) -> bool {
    ops.iter().any(|op| match op {
        tune_plan::PlanOp::Match { .. } => true,
        tune_plan::PlanOp::If {
            branches, else_ops, ..
        } => {
            branches.iter().any(|branch| {
                plan_ops_contain_match(&branch.condition_ops)
                    || plan_ops_contain_match(&branch.body_ops)
            }) || plan_ops_contain_match(else_ops)
        }
        tune_plan::PlanOp::FiniteFor {
            iterable_ops,
            body_ops,
            ..
        }
        | tune_plan::PlanOp::While {
            condition_ops: iterable_ops,
            body_ops,
            ..
        } => plan_ops_contain_match(iterable_ops) || plan_ops_contain_match(body_ops),
        tune_plan::PlanOp::Loop { body_ops, .. } => plan_ops_contain_match(body_ops),
        tune_plan::PlanOp::BoolAnd {
            lhs_ops, rhs_ops, ..
        }
        | tune_plan::PlanOp::BoolOr {
            lhs_ops, rhs_ops, ..
        } => plan_ops_contain_match(lhs_ops) || plan_ops_contain_match(rhs_ops),
        _ => false,
    })
}

#[test]
fn struct_construct_plan_carries_local_state_plan() -> Result<(), &'static str> {
    let source = r#"
struct Counter {
  value: Int
}
let result: Counter = Counter {
  value = 1
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let plan = tune_plan::lower_resolved_module_item_to_plan(&module, &module.items[1], &resolved)
        .ok_or("expected result plan")?;

    assert!(plan.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::StructConstruct {
            escape: tune_plan::StructEscapeReason::Local,
            state: tune_plan::StructStatePlan::LOCAL,
            ..
        }
    )));

    Ok(())
}

#[test]
fn struct_state_decision_records_escape_reason() {
    let local = tune_plan::StructStateDecision::for_escape(tune_plan::StructEscapeReason::Local);
    assert_eq!(local.reason, tune_plan::StructEscapeReason::Local);
    assert_eq!(local.plan, tune_plan::StructStatePlan::LOCAL);

    let spawned =
        tune_plan::StructStateDecision::for_escape(tune_plan::StructEscapeReason::SpawnBoundary);
    assert_eq!(spawned.reason, tune_plan::StructEscapeReason::SpawnBoundary);
    assert_eq!(spawned.plan, tune_plan::StructStatePlan::SHARED);
}

#[test]
fn module_plan_entry_runs_top_level_values_in_order() -> Result<(), &'static str> {
    let source = "let a: Int = 1\nlet b: Int = a + 2\nlet helper(): Int = 99";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let plan = tune_plan::lower_resolved_module_to_plan(&module, &resolved);
    let entry = plan.entry.ok_or("module entry should exist")?;

    assert_eq!(entry.name, "<entry>");
    assert_eq!(
        entry.module_bindings,
        vec![tune_hir::HirId(0), tune_hir::HirId(1)]
    );
    assert_eq!(plan.functions.len(), 1);
    assert!(entry.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::ModuleLet {
            item: tune_hir::HirId(0),
            ..
        }
    )));
    assert!(entry.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::BindingGet {
            source: Some(tune_resolve::NameTarget::TopLevel(tune_hir::HirId(0)))
        }
    )));

    Ok(())
}

#[test]
fn semantic_plan_has_typed_materialization_and_meta_slots() {
    let materialize = tune_plan::PlanOp::Materialize {
        plan: tune_shape::MaterializationPlan {
            target: tune_shape::Shape::Sequence(Box::new(tune_shape::Shape::Int)),
            commitment: tune_shape::Commitment::PerUse,
        },
        materializer: None,
    };
    assert!(matches!(
        materialize,
        tune_plan::PlanOp::Materialize {
            plan: tune_shape::MaterializationPlan {
                commitment: tune_shape::Commitment::PerUse,
                ..
            },
            materializer: None,
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
