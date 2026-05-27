#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn optimizer_runs_ordered_semantic_passes_over_ir() {
    let mut function = tune_ir::IrFunction {
        name: "run".into(),
        regs: 0,
        locals: 0,
        constants: Vec::new(),
        blocks: Vec::new(),
    };

    let report = tune_opt::optimize(&mut function);
    let passes = report
        .passes
        .iter()
        .map(|pass| pass.pass)
        .collect::<Vec<_>>();

    assert_eq!(
        passes,
        vec![
            tune_opt::Pass::Escape,
            tune_opt::Pass::ThreadEscape,
            tune_opt::Pass::RcElim,
            tune_opt::Pass::BoundsCheckElim,
            tune_opt::Pass::Generics,
            tune_opt::Pass::Strings,
        ]
    );
    assert!(report.passes.iter().all(|pass| !pass.changed));
}
