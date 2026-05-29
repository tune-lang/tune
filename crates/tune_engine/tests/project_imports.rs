#[test]
fn engine_loads_and_runs_manifest_entry_source() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/helper.tn".to_owned(),
                    "pub let ignored: Int = 1".to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    "pub let result: Int = 40 + 2".to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry)
            .map_err(|_| "project entry should run")?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_can_import_member_from_loaded_source() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    "pub let add(a: Int, b: Int): Int = a + b".to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".add
let result: Int = add(20, 22)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry)
            .map_err(|_| "project entry should run")?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_imports_selected_member_dependencies() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    r#"
let inc(value: Int): Int = value + 1
pub let add_next(a: Int, b: Int): Int = inc(a) + b
"#
                    .to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".add_next
let result: Int = add_next(19, 22)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry).map_err(|error| {
            eprintln!("{error:?}");
            "project entry should run"
        })?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_imports_selected_member_recursive_dependencies() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/base.tn".to_owned(),
                    "pub let inc(value: Int): Int = value + 1".to_owned(),
                ),
                (
                    "src/math.tn".to_owned(),
                    r#"
import "src/base.tn".inc
pub let add_next(a: Int, b: Int): Int = inc(a) + b
"#
                    .to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".add_next
let result: Int = add_next(19, 22)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry).map_err(|error| {
            eprintln!("{error:?}");
            "project entry should run"
        })?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_import_dependencies_do_not_leak_selected_scope() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    r#"
let inc(value: Int): Int = value + 1
pub let add_next(a: Int, b: Int): Int = inc(a) + b
"#
                    .to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".add_next
let inc(value: Int): Int = 100
let result: Int = add_next(19, 22)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry).map_err(|error| {
            eprintln!("{error:?}");
            "project entry should run"
        })?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_can_import_source_module_namespace() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    r#"
let inc(value: Int): Int = value + 1
pub let add_next(a: Int, b: Int): Int = inc(a) + b
"#
                    .to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn"
let result: Int = math.add_next(19, 22)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    assert_eq!(
        tune.run_project_entry(entry).map_err(|error| {
            eprintln!("{error:?}");
            "project entry should run"
        })?,
        tune_runtime::Value::Int(42)
    );
    Ok(())
}

#[test]
fn project_entry_rejects_private_module_namespace_member() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    "let helper(value: Int): Int = value + 1".to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn"
let result: Int = math.helper(41)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_project_entry(entry)
    else {
        return Err("private module member should stop execution");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::UNRESOLVED_NAME
            && diagnostic.title == "unresolved module member `helper`"
    }));

    Ok(())
}

#[test]
fn project_entry_reports_unresolved_import_members() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    "pub let add(a, b) = a + b".to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".missing
let result = missing(1, 2)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_project_entry(entry)
    else {
        return Err("unresolved import member should stop execution");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::UNRESOLVED_NAME
            && diagnostic.title == "unresolved import member `missing`"
    }));

    Ok(())
}

#[test]
fn project_entry_rejects_private_selected_import_member() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new();
    let entry = tune
        .load_project_sources(
            dyno_project::Manifest::new("app", "src/app.tn"),
            vec![
                (
                    "src/math.tn".to_owned(),
                    "let helper(value: Int): Int = value + 1".to_owned(),
                ),
                (
                    "src/app.tn".to_owned(),
                    r#"
import "src/math.tn".helper
let result: Int = helper(41)
"#
                    .to_owned(),
                ),
            ],
        )
        .map_err(|_| "project sources should load")?;

    let Err(tune_engine::EngineError::Diagnostics(diagnostics)) = tune.run_project_entry(entry)
    else {
        return Err("private import member should stop execution");
    };
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::IMPORT_NOT_VISIBLE
            && diagnostic.title == "import member `helper` is private"
    }));

    Ok(())
}
