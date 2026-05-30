#[test]
fn registry_resolves_exact_typed_package_versions() {
    let mut registry = dyno_pkg::Registry::new();
    let package = dyno_pkg::Package::new("json", "1.0.0", "sha256:abc");

    assert!(registry.publish(package));
    assert!(!registry.publish(dyno_pkg::Package::new("json", "1.0.0", "sha256:abc")));

    let resolved = registry.resolve(
        &dyno_project::PackageRef {
            name: "json".into(),
        },
        &dyno_project::VersionReq("1.0.0".into()),
    );
    assert!(resolved.is_some());
    assert!(
        registry
            .resolve(
                &dyno_project::PackageRef {
                    name: "json".into(),
                },
                &dyno_project::VersionReq("2.0.0".into()),
            )
            .is_none()
    );
}
