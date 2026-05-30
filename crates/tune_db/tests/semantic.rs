fn offset_of(source: &str, needle: &str) -> Result<tune_diagnostics::ByteOffset, &'static str> {
    let offset = source.find(needle).ok_or("fixture should contain needle")?;
    Ok(tune_diagnostics::ByteOffset::new(
        offset.try_into().map_err(|_| "fixture offset fits")?,
    ))
}

#[test]
fn semantic_cursor_reports_reference_definition_shape_and_call_context() -> Result<(), &'static str>
{
    let source = r#"
let add(a: Int, b: Int): Int = a + b
let value: Int = add(1, 2)
"#;
    let mut db = tune_db::TuneDb::new();
    let file = db
        .add_file("main.tn", source)
        .ok_or("source file should allocate")?;

    let cursor = db
        .semantic_at(file, offset_of(source, "add(1")?)
        .ok_or("semantic cursor should resolve")?;

    assert_eq!(cursor.owner, Some(tune_hir::HirId(1)));
    assert_eq!(
        cursor.reference.as_ref().map(|reference| reference.target),
        Some(tune_resolve::NameTarget::TopLevel(tune_hir::HirId(0)))
    );
    assert_eq!(
        cursor
            .reference
            .as_ref()
            .and_then(|reference| reference.definition.as_ref())
            .and_then(|definition| definition.name.as_deref()),
        Some("add")
    );
    assert_eq!(cursor.call.as_ref().and_then(|call| call.active_arg), None);

    let arg_cursor = db
        .semantic_at(file, offset_of(source, "2)")?)
        .ok_or("semantic cursor should resolve argument")?;
    assert_eq!(
        arg_cursor.call.as_ref().and_then(|call| call.active_arg),
        Some(1)
    );
    assert_eq!(
        arg_cursor
            .expr
            .as_ref()
            .and_then(|expr| expr.shape.as_ref()),
        Some(&tune_shape::Shape::Int)
    );

    Ok(())
}

#[test]
fn semantic_cursor_reports_scope_bindings_for_local_tooling() -> Result<(), &'static str> {
    let source = r#"
let outer: Int = 1
let run(input: Int): Int = {
  let local: Int = input
  local
}
"#;
    let mut db = tune_db::TuneDb::new();
    let file = db
        .add_file("main.tn", source)
        .ok_or("source file should allocate")?;

    let cursor = db
        .semantic_at(file, offset_of(source, "local\n}")?)
        .ok_or("semantic cursor should resolve")?;
    let names = cursor
        .scope
        .iter()
        .map(|binding| binding.name.as_str())
        .collect::<Vec<_>>();

    assert!(names.contains(&"outer"));
    assert!(names.contains(&"run"));
    assert!(names.contains(&"input"));
    assert!(names.contains(&"local"));

    Ok(())
}
