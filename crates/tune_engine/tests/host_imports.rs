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

struct MetaHost;

impl tune_host::Host for MetaHost {
    fn modules(&self) -> Vec<tune_host::HostModule> {
        vec![
            tune_host::HostModule::new(
                "meta",
                vec![
                    tune_host::HostFunction::new(
                        "make",
                        Vec::new(),
                        tune_shape::Shape::Struct(tune_shape::NominalShape::external("meta.Pair")),
                    )
                    .with_executor(|_: &[tune_runtime::Value]| {
                        Ok(tune_runtime::Value::HostStruct {
                            type_name: "meta.Pair".into(),
                            fields: vec![
                                ("count".into(), tune_runtime::Value::Int(42)),
                                ("name".into(), tune_runtime::Value::String("answer".into())),
                            ],
                        })
                    }),
                ],
            )
            .with_values(vec![tune_host::HostValueType::new(
                "Pair",
                vec![
                    tune_host::HostValueField::new("count", tune_shape::Shape::Int),
                    tune_host::HostValueField::new("name", tune_shape::Shape::String),
                ],
            )]),
        ]
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

#[test]
fn host_value_structs_flow_through_shape_plan_and_vm() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_host(&MetaHost);
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "meta".make
let result: Int = make().count
"#,
        )
        .ok_or("file should allocate")?;

    let value = tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "host value struct should execute"
    })?;

    assert_eq!(value, tune_runtime::Value::Int(42));
    Ok(())
}

#[test]
fn host_value_struct_shape_flows_through_top_level_binding() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_host(&MetaHost);
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "meta".make
let pair = make()
let result: Int = pair.count
"#,
        )
        .ok_or("file should allocate")?;

    let value = tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "host value struct should keep shape through top-level binding"
    })?;

    assert_eq!(value, tune_runtime::Value::Int(42));
    Ok(())
}

#[test]
fn host_resource_shapes_are_available_to_namespace_function_signatures() -> Result<(), &'static str>
{
    let mut tune = tune_engine::Tune::new().with_std();
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "fs"
let result = fs.open("missing.txt")
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_file(file).ok_or("file should check")?;
    if !check.diagnostics.is_empty() {
        eprintln!("{:?}", check.diagnostics);
        return Err("fs namespace import should resolve resource-shaped signatures");
    }
    Ok(())
}

#[test]
fn host_resource_types_do_not_become_namespace_value_members() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_std();
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "fs"
let result = fs.File
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_file(file).ok_or("file should check")?;
    assert!(check.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::UNRESOLVED_NAME
            && diagnostic.title == "unresolved module member `File`"
    }));
    Ok(())
}
