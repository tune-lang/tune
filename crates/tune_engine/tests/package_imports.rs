#[test]
fn project_entry_imports_locked_package_root_module() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let mut manifest = dyno_project::Manifest::new("app", "src/app.tn");
    manifest.dependencies.push(json_dependency());
    let lockfile = json_lockfile();
    let entry = tune
        .load_project_sources_with_packages(
            manifest,
            &lockfile,
            vec![(
                "src/app.tn".to_owned(),
                r#"
import "json".answer
let result: Int = answer()
"#
                .to_owned(),
            )],
            vec![tune_engine::ProjectPackageSources::new(
                dyno_project::PackageRef {
                    name: "json".into(),
                },
                vec![(
                    "src/lib.tn".to_owned(),
                    "pub let answer(): Int = 42".to_owned(),
                )],
            )],
        )
        .map_err(|error| {
            eprintln!("{error:?}");
            "project with package should load"
        })?;

    assert_eq!(
        tune.run_project_entry(entry)
            .map_err(|_| "project should run")?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_package_import_requires_lockfile_entry() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let mut manifest = dyno_project::Manifest::new("app", "src/app.tn");
    manifest.dependencies.push(json_dependency());

    let Err(tune_engine::EngineError::ProjectLoad(message)) = tune
        .load_project_sources_with_packages(
            manifest,
            &dyno_project::Lockfile::new(),
            vec![("src/app.tn".to_owned(), "let result: Int = 1".to_owned())],
            Vec::new(),
        )
    else {
        return Err("missing package lock should reject project load");
    };
    assert!(message.contains("missing from dyno.lock"));
    Ok(())
}

#[test]
fn project_rejects_sources_for_unlocked_package() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let manifest = dyno_project::Manifest::new("app", "src/app.tn");

    let Err(tune_engine::EngineError::ProjectLoad(message)) = tune
        .load_project_sources_with_packages(
            manifest,
            &dyno_project::Lockfile::new(),
            vec![("src/app.tn".to_owned(), "let result: Int = 1".to_owned())],
            vec![tune_engine::ProjectPackageSources::new(
                dyno_project::PackageRef {
                    name: "json".into(),
                },
                vec![(
                    "src/lib.tn".to_owned(),
                    "pub let answer: Int = 42".to_owned(),
                )],
            )],
        )
    else {
        return Err("unlocked package source should reject project load");
    };
    assert!(message.contains("not a locked dependency"));
    Ok(())
}

fn json_dependency() -> dyno_project::Dependency {
    dyno_project::Dependency {
        package: dyno_project::PackageRef {
            name: "json".into(),
        },
        requirement: dyno_project::VersionReq("1.0.0".into()),
    }
}

fn json_lockfile() -> dyno_project::Lockfile {
    let mut lockfile = dyno_project::Lockfile::new();
    assert!(lockfile.add(dyno_project::LockedPackage {
        package: dyno_project::PackageRef {
            name: "json".into(),
        },
        version: dyno_project::VersionReq("1.0.0".into()),
        checksum: dyno_project::Checksum("sha256:json".into()),
        source: dyno_project::PackageSource::Registry("dyno".into()),
    }));
    lockfile
}
