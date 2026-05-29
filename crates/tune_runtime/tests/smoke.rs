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
    assert_ne!(
        tune_runtime::TaskExecutionMode::Immediate,
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

#[test]
fn resource_handles_are_task_unsafe_by_default() {
    let resource = tune_runtime::Value::Resource(tune_runtime::ResourceHandle::new(
        tune_runtime::ResourceId(1),
        "fs.File",
    ));

    assert_eq!(
        resource.task_safety_error(),
        Some(tune_runtime::TaskSafetyError {
            resource_type: "fs.File".into(),
        })
    );
    assert!(!resource.is_task_safe());
}

#[test]
fn resource_task_safety_checks_nested_values() {
    let resource = tune_runtime::Value::Resource(
        tune_runtime::ResourceHandle::new(tune_runtime::ResourceId(2), "net.Socket")
            .task_safe(true),
    );
    let value = tune_runtime::Value::Tuple(vec![
        tune_runtime::Value::Int(1),
        tune_runtime::Value::Sequence(vec![resource]),
    ]);

    assert!(value.is_task_safe());
}
