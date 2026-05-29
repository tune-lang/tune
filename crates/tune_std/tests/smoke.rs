#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn stdcore_registry_includes_auto_included_core_shapes() {
    let registry = tune_std::prelude::stdcore();

    assert!(
        registry
            .shapes
            .contains(&tune_std::prelude::StdCoreShape::Result)
    );
    assert!(
        registry
            .shapes
            .contains(&tune_std::prelude::StdCoreShape::Map)
    );
    assert!(
        registry
            .shapes
            .contains(&tune_std::prelude::StdCoreShape::Set)
    );
}
