#[test]
fn frontend_profile_skips_bytecode_and_collects_ir_quality() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_std();
    let file = tune
        .add_file(
            "case.tn",
            r#"
let a: Int = 10
let b: Int = 20
let c: Int = a + b
let result: Int = if c > 10 { c } else { a }
"#,
        )
        .ok_or("source should allocate")?;

    let report = tune
        .profile_file_frontend(file)
        .map_err(|_| "profile should succeed")?;

    assert_eq!(report.bytecode.instructions, 0);
    assert_eq!(report.bytecode.functions, 0);
    assert_eq!(
        report.stop_reason.as_deref(),
        Some("frontend profiling skipped bytecode")
    );
    assert_eq!(report.ir.shape_holes, 0);
    assert!(report.ir.ops > 0);

    Ok(())
}
