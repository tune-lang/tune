#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn task_join_preserves_pending_and_ready_states() {
    let ready = tune_runtime::Task::ready(tune_runtime::TaskId(1), tune_runtime::Value::Int(20));
    assert_eq!(
        ready.join(),
        tune_runtime::TaskJoinOutcome::Ready(tune_runtime::Value::Int(20))
    );

    let pending = tune_runtime::Task::pending(tune_runtime::TaskId(2));
    assert_eq!(
        pending.join(),
        tune_runtime::TaskJoinOutcome::Pending(tune_runtime::TaskId(2))
    );
}

#[test]
fn task_execution_mode_names_scheduler_boundary() {
    assert_eq!(
        tune_runtime::TaskExecutionMode::DeferredUntilJoin,
        tune_runtime::TaskExecutionMode::DeferredUntilJoin
    );
}

#[test]
fn task_join_does_not_convert_panics_to_recoverable_errors() {
    let panicked = tune_runtime::Task::panicked(
        tune_runtime::TaskId(3),
        tune_runtime::TunePanic {
            message: "bad".into(),
        },
    );

    assert_eq!(
        panicked.join(),
        tune_runtime::TaskJoinOutcome::UnrecoverablePanic(tune_runtime::TunePanic {
            message: "bad".into(),
        })
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
