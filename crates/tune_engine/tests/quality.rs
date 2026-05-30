#[test]
fn frontend_profile_skips_bytecode_and_collects_ir_quality() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_std();
    let file = tune
        .add_source(
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
        .profile_source_frontend(file)
        .map_err(|_| "profile should succeed")?;

    assert_eq!(report.bytecode.instructions, 0);
    assert_eq!(report.bytecode.functions, 0);
    assert_eq!(
        report.stop_reason.as_deref(),
        Some("frontend profiling skipped bytecode")
    );
    assert_eq!(report.ir.shape_holes, 0);
    assert!(report.ir.ops > 0);
    assert_eq!(report.ir.bound_calls, 0);
    assert_eq!(report.ir.witness_calls, 0);
    assert!(report.ir.local_reads > 0);
    assert!(report.ir.local_inits > 0);
    assert!(report.ir.transfer_copies > 0);

    Ok(())
}

#[test]
fn full_profile_reports_backend_guard_pressure() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_std();
    let file = tune
        .add_source(
            "case.tn",
            r#"
let values: [Int] = [1, 2, 3]
let result: Int = values[1]
"#,
        )
        .ok_or("source should allocate")?;

    let report = tune
        .profile_source(file)
        .map_err(|_| "profile should succeed")?;

    assert!(report.bytecode.instructions > 0);
    assert_eq!(report.plan.dynamic_bound_calls, 0);
    assert_eq!(report.ir.shape_holes, 0);
    assert_eq!(report.ir.sequence_builds, 1);
    assert_eq!(report.ir.sequence_pushes, 3);
    assert_eq!(report.ir.exclusive_sequence_mutations, 3);
    assert_eq!(report.ir.shared_cow_sequence_mutations, 0);
    assert_eq!(report.bytecode.bound_calls, 0);
    assert_eq!(report.bytecode.runtime_type_guard_pressure, 0);
    assert_eq!(report.bytecode.checked_sequence_ops, 0);
    assert!(report.bytecode.unchecked_sequence_ops > 0);

    Ok(())
}
