#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn optimizer_runs_ordered_semantic_passes_over_ir() {
    let mut function = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        span: None,
        regs: 0,
        locals: 0,
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        blocks: Vec::new(),
        task_functions: Vec::new(),
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
    assert_eq!(report.ownership, tune_opt::OwnershipReport::default());
}

#[test]
fn optimizer_reports_struct_ownership_facts_from_ir() {
    let function = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        span: None,
        regs: 1,
        locals: 0,
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![tune_ir::IrOp::StructConstruct {
                dst: tune_ir::Reg(0),
                item: tune_hir::HirId(1),
                state: tune_ir::IrStructState {
                    repr: tune_ir::IrStateRepr::SharedHandle,
                    ownership: tune_ir::IrOwnershipPlan::SharedAtomic,
                },
                fields: Vec::new(),
                span: None,
            }],
        }],
        task_functions: Vec::new(),
    };

    let report = tune_opt::ownership_report(&function);
    assert_eq!(report.shared_atomic, 1);
    assert_eq!(report.non_atomic_rc, 0);
}
