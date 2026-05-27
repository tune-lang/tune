#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn host_functions_use_typed_shapes_not_signature_strings() {
    let function = tune_host::HostFunction::new(
        "read",
        vec![tune_host::HostParam::new("path", tune_shape::Shape::String)],
        tune_shape::Shape::Result {
            ok: Box::new(tune_shape::Shape::String),
            err: Box::new(tune_shape::Shape::Struct("FsError".into())),
        },
    )
    .with_authorities(vec![tune_host::Authority("fs.read".into())])
    .task_safe(true);

    assert_eq!(function.name, "read");
    assert_eq!(function.params[0].shape, tune_shape::Shape::String);
    assert!(matches!(function.ret, tune_shape::Shape::Result { .. }));
    assert_eq!(function.authorities[0].0, "fs.read");
    assert!(function.task_safe);
}
