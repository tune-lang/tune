struct PathHost;

impl tune_host::Host for PathHost {
    fn modules(&self) -> Vec<tune_host::HostModule> {
        vec![tune_host::HostModule::new(
            "path",
            vec![
                tune_host::HostFunction::new(
                    "join",
                    vec![
                        tune_host::HostParam::new("base", tune_shape::Shape::String),
                        tune_host::HostParam::new("name", tune_shape::Shape::String),
                    ],
                    tune_shape::Shape::String,
                )
                .with_executor(|args: &[tune_runtime::Value]| {
                    let [
                        tune_runtime::Value::String(base),
                        tune_runtime::Value::String(name),
                    ] = args
                    else {
                        return Err(tune_host::HostCallError::new(
                            "path.join expected two String arguments",
                        ));
                    };
                    Ok(tune_runtime::Value::String(format!("{base}/{name}")))
                }),
            ],
        )]
    }
}

#[test]
fn host_module_import_exposes_namespace_members() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_host(&PathHost);
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "path"
let result: String = path.join("src", "main.tn")
"#,
        )
        .ok_or("file should allocate")?;

    let value = tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "host namespace import should execute"
    })?;

    assert_eq!(value, tune_runtime::Value::String("src/main.tn".into()));
    Ok(())
}

#[test]
fn host_module_import_does_not_leak_members_to_top_level() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_host(&PathHost);
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "path"
let result: String = join("src", "main.tn")
"#,
        )
        .ok_or("file should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
        return Err("unqualified host module member should not resolve");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::UNRESOLVED_NAME
            && diagnostic.title == "unresolved name `join`"
    }));
    Ok(())
}
