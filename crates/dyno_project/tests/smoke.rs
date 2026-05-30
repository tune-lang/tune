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
    assert!(resolution.missing_dependencies.is_empty());
    assert!(
        resolution
            .roots
            .contains(&dyno_project::ModuleRoot::Package(
                dyno_project::PackageRef {
                    name: "json".into()
                }
            ))
    );
    assert!(resolution.roots.contains(&dyno_project::ModuleRoot::Std));
    assert!(
        resolution
            .roots
            .contains(&dyno_project::ModuleRoot::Host("fs".into()))
    );
}

#[test]
fn project_manifest_round_trips_minimal_toml() -> Result<(), &'static str> {
    let manifest = dyno_project::Manifest::from_toml(
        r#"
[project]
name = "demo"
edition = "2026"
entry = "src/main.tn"
strict = false

[dependencies]
json = "1.0.0"

[host]
profile = "dyno.default"
"#,
    )
    .map_err(|_| "manifest should parse")?;

    assert_eq!(manifest.name, "demo");
    assert_eq!(
        manifest.entry,
        dyno_project::ModulePath("src/main.tn".to_owned())
    );
    assert!(
        manifest
            .roots
            .contains(&dyno_project::ModuleRoot::Source(dyno_project::ModulePath(
                "src".to_owned()
            )))
    );
    assert!(
        manifest
            .roots
            .contains(&dyno_project::ModuleRoot::Host("dyno.default".to_owned()))
    );
    assert_eq!(
        manifest.dependencies,
        vec![dyno_project::Dependency {
            package: dyno_project::PackageRef {
                name: "json".into()
            },
            requirement: dyno_project::VersionReq("1.0.0".into())
        }]
    );
    assert!(manifest.to_toml().contains("edition = \"2026\""));
    assert!(manifest.to_toml().contains("json = \"1.0.0\""));

    Ok(())
}

#[test]
fn project_resolution_reports_missing_dependency_locks() {
    let mut manifest = dyno_project::Manifest::new("app", "main.tn");
    manifest.dependencies.push(dyno_project::Dependency {
        package: dyno_project::PackageRef {
            name: "json".into(),
        },
        requirement: dyno_project::VersionReq("1.0.0".into()),
    });

    let resolution = dyno_project::resolve(&manifest, &dyno_project::Lockfile::new());

    assert_eq!(resolution.locked_package_count, 0);
    assert_eq!(resolution.missing_dependencies, manifest.dependencies);
    assert!(
        !resolution
            .roots
            .contains(&dyno_project::ModuleRoot::Package(
                dyno_project::PackageRef {
                    name: "json".into()
                }
            ))
    );
}

#[test]
fn loads_project_sources_from_manifest_path() -> Result<(), String> {
    let root =
        std::env::temp_dir().join(format!("dyno-project-source-load-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
    }
    std::fs::create_dir_all(root.join("src")).map_err(|error| error.to_string())?;
    let manifest = dyno_project::Manifest::new("demo", "src/main.tn");
    std::fs::write(root.join("dyno.toml"), manifest.to_toml())
        .map_err(|error| error.to_string())?;
    std::fs::write(root.join("src/main.tn"), "let value: Int = 42")
        .map_err(|error| error.to_string())?;

    let loaded = dyno_project::load_project_manifest(root.join("dyno.toml"))
        .map_err(|error| format!("{error:?}"))?;
    std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;

    assert_eq!(loaded.manifest.name, "demo");
    assert!(loaded.sources.iter().any(|(path, _)| path == "src/main.tn"));

    Ok(())
}
