fn lower_callable(source: &str) -> Result<tune_plan::PlanFunction, &'static str> {
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    tune_plan::lower_resolved_module_item_to_plan(&module, &module.items[1], &resolved)
        .ok_or("callable should lower")
}

#[test]
fn implicit_struct_return_records_return_escape() -> Result<(), &'static str> {
    let plan = lower_callable(
        r#"
struct Counter {
  value: Int
}
let make(): Counter = Counter {
  value = 1
}
"#,
    )?;

    assert!(plan.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::StructConstruct {
            escape: tune_plan::StructEscapeReason::Returned,
            state: tune_plan::StructStatePlan::LOCAL,
            ..
        }
    )));

    Ok(())
}

#[test]
fn explicit_struct_return_records_return_escape() -> Result<(), &'static str> {
    let plan = lower_callable(
        r#"
struct Counter {
  value: Int
}
let make(): Counter = return Counter {
  value = 1
}
"#,
    )?;

    assert!(plan.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::StructConstruct {
            escape: tune_plan::StructEscapeReason::Returned,
            state: tune_plan::StructStatePlan::LOCAL,
            ..
        }
    )));

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
fn callable_value_captures_mark_struct_binding_escape() -> Result<(), &'static str> {
    let plan = lower_callable(
        r#"
struct Counter {
  value: Int
}
let make(seed: Int) = {
  let counter: Counter = Counter {
    value = seed
  }
  _(amount: Int) = {
    counter.value = counter.value + amount
    counter.value
  }
}
"#,
    )?;

    assert!(plan.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::StructConstruct {
            escape: tune_plan::StructEscapeReason::Captured,
            state: tune_plan::StructStatePlan::LOCAL,
            ..
        }
    )));
    assert!(plan.ops.iter().any(|op| matches!(
        op,
        tune_plan::PlanOp::CallableValue { captures } if !captures.is_empty()
    )));

    Ok(())
}
