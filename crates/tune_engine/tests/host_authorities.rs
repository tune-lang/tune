struct SecretHost;

impl tune_host::Host for SecretHost {
    fn modules(&self) -> Vec<tune_host::HostModule> {
        vec![tune_host::HostModule::new(
            "secret",
            vec![
                tune_host::HostFunction::new("answer", Vec::new(), tune_shape::Shape::Int)
                    .with_authorities(vec![tune_host::Authority("secret.read".into())])
                    .with_executor(|_: &[tune_runtime::Value]| Ok(tune_runtime::Value::Int(42))),
            ],
        )]
    }
}

#[test]
fn engine_rejects_host_call_without_required_authority() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_host(&SecretHost);
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "secret".answer
let result: Int = answer()
"#,
        )
        .ok_or("source should allocate")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_file(file) else {
        return Err("host call should require authority");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic
            .facts
            .iter()
            .flat_map(|fact| &fact.entries)
            .any(|entry| entry.message.contains("secret.read"))
    }));

    Ok(())
}

#[test]
fn engine_executes_host_call_with_required_authority() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new()
        .with_host(&SecretHost)
        .with_authority(tune_host::Authority("secret.read".into()));
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "secret".answer
let result: Int = answer()
"#,
        )
        .ok_or("source should allocate")?;

    assert_eq!(
        tune.run_file(file).map_err(|error| {
            eprintln!("{error:?}");
            "host call should execute"
        })?,
        tune_runtime::Value::Int(42)
    );

    Ok(())
}
