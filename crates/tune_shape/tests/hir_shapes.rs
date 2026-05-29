#[test]
fn resolved_hir_shape_lowers_stdcore_generics_and_bare_holes() -> Result<(), &'static str> {
    let source = r#"
struct Config {}
enum ParseError {}
let parse(text: String): Result<Config, ParseError> = text
let background(): Task<Result<Config, ParseError>> = parse("")
let bare_result: Result = none
let bare_task: Task = none
let bare_map: Map = none
let bare_set: Set = none
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);

    let result_shape = module.items[2]
        .shape
        .as_ref()
        .ok_or("expected result shape")?;
    let task_shape = module.items[3]
        .shape
        .as_ref()
        .ok_or("expected task shape")?;
    let bare_result_shape = module.items[4]
        .shape
        .as_ref()
        .ok_or("expected bare result shape")?;
    let bare_task_shape = module.items[5]
        .shape
        .as_ref()
        .ok_or("expected bare task shape")?;
    let bare_map_shape = module.items[6]
        .shape
        .as_ref()
        .ok_or("expected bare map shape")?;
    let bare_set_shape = module.items[7]
        .shape
        .as_ref()
        .ok_or("expected bare set shape")?;

    let lowered_result = tune_shape::lower_resolved_hir_shape(result_shape, &resolved.scope);
    let lowered_task = tune_shape::lower_resolved_hir_shape(task_shape, &resolved.scope);
    let lowered_bare_result =
        tune_shape::lower_resolved_hir_shape(bare_result_shape, &resolved.scope);
    let lowered_bare_task = tune_shape::lower_resolved_hir_shape(bare_task_shape, &resolved.scope);
    let lowered_bare_map = tune_shape::lower_resolved_hir_shape(bare_map_shape, &resolved.scope);
    let lowered_bare_set = tune_shape::lower_resolved_hir_shape(bare_set_shape, &resolved.scope);

    assert!(lowered_result.diagnostics.is_empty());
    assert!(lowered_task.diagnostics.is_empty());
    assert!(lowered_bare_result.diagnostics.is_empty());
    assert!(lowered_bare_task.diagnostics.is_empty());
    assert!(lowered_bare_map.diagnostics.is_empty());
    assert!(lowered_bare_set.diagnostics.is_empty());
    assert!(matches!(
        lowered_result.shape,
        tune_shape::Shape::Result { .. }
    ));
    assert!(matches!(lowered_task.shape, tune_shape::Shape::Task(_)));
    assert!(matches!(
        lowered_bare_result.shape,
        tune_shape::Shape::Result { ok, err }
            if *ok == tune_shape::Shape::Hole && *err == tune_shape::Shape::Hole
    ));
    assert!(matches!(
        lowered_bare_task.shape,
        tune_shape::Shape::Task(inner) if *inner == tune_shape::Shape::Hole
    ));
    assert!(matches!(
        lowered_bare_map.shape,
        tune_shape::Shape::Apply { nominal, args }
            if nominal.name == "Map" && args == vec![tune_shape::Shape::Hole, tune_shape::Shape::Hole]
    ));
    assert!(matches!(
        lowered_bare_set.shape,
        tune_shape::Shape::Apply { nominal, args }
            if nominal.name == "Set" && args == vec![tune_shape::Shape::Hole]
    ));

    Ok(())
}
