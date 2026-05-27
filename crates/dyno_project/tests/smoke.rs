#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn project_manifest_lockfile_and_resolution_are_typed() {
    let mut manifest = dyno_project::Manifest::new("app", "main.tn");
    manifest.dependencies.push(dyno_project::Dependency {
        package: dyno_project::PackageRef {
            name: "json".into(),
        },
        requirement: dyno_project::VersionReq("1.0.0".into()),
    });
    manifest
        .roots
        .push(dyno_project::ModuleRoot::Host("fs".into()));

    let mut lockfile = dyno_project::Lockfile::new();
    assert!(lockfile.add(dyno_project::LockedPackage {
        package: dyno_project::PackageRef {
            name: "json".into(),
        },
        version: dyno_project::VersionReq("1.0.0".into()),
        checksum: dyno_project::Checksum("sha256:abc".into()),
        source: dyno_project::PackageSource::Registry("dyno".into()),
    }));

    let resolution = dyno_project::resolve(&manifest, &lockfile);
    assert_eq!(resolution.locked_package_count, 1);
    assert!(resolution.roots.contains(&dyno_project::ModuleRoot::Std));
    assert!(
        resolution
            .roots
            .contains(&dyno_project::ModuleRoot::Host("fs".into()))
    );
}
