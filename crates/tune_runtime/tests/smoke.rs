#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn task_join_preserves_pending_ready_and_panic_states() {
    let ready = tune_runtime::Task::ready(tune_runtime::TaskId(1), tune_runtime::Value::Int(20));
    assert_eq!(ready.join(), Ok(tune_runtime::Value::Int(20)));

    let pending = tune_runtime::Task::pending(tune_runtime::TaskId(2));
    assert_eq!(
        pending.join(),
        Err(tune_runtime::TaskJoinError::Pending(tune_runtime::TaskId(
            2
        )))
    );

    let panic = tune_runtime::TunePanic {
        message: "bad".into(),
    };
    let panicked = tune_runtime::Task::panicked(tune_runtime::TaskId(3), panic.clone());
    assert_eq!(
        panicked.join(),
        Err(tune_runtime::TaskJoinError::Panicked(panic))
    );
}

#[test]
fn state_handles_record_repr_and_ownership_cost() {
    let local = tune_runtime::StateHandle::local(tune_runtime::StateId(1));
    assert_eq!(local.repr, tune_runtime::StateRepr::LocalHandle);
    assert_eq!(
        local.ownership,
        tune_runtime::ownership::OwnershipPlan::NonAtomicRc
    );

    let shared = tune_runtime::StateHandle::shared(tune_runtime::StateId(2));
    assert_eq!(shared.repr, tune_runtime::StateRepr::SharedHandle);
    assert_eq!(
        shared.ownership,
        tune_runtime::ownership::OwnershipPlan::SharedAtomic
    );
}
