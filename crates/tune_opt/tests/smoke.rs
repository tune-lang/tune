#[test]
fn optimizer_runs_ordered_semantic_passes_over_ir() {
    let mut function = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        type_params: Vec::new(),
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
        type_params: Vec::new(),
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

#[test]
fn optimizer_eliminates_proven_sequence_bounds_checks() {
    let mut function = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        type_params: Vec::new(),
        span: None,
        regs: 4,
        locals: 1,
        constants: vec![
            tune_ir::IrConst::Int(1),
            tune_ir::IrConst::Int(2),
            tune_ir::IrConst::Size(1),
        ],
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::SeqBuild {
                    dst: tune_ir::Reg(0),
                    element_shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(1),
                    constant: tune_ir::ConstId(0),
                    shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::SeqPush {
                    seq: tune_ir::Reg(0),
                    value: tune_ir::Reg(1),
                },
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(1),
                    constant: tune_ir::ConstId(1),
                    shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::SeqPush {
                    seq: tune_ir::Reg(0),
                    value: tune_ir::Reg(1),
                },
                tune_ir::IrOp::StoreLocal {
                    local: tune_ir::LocalId(0),
                    value: tune_ir::Reg(0),
                    store: tune_ir::IrLocalStore::Init,
                },
                tune_ir::IrOp::LoadLocal {
                    dst: tune_ir::Reg(0),
                    local: tune_ir::LocalId(0),
                    access: tune_ir::IrLocalAccess::Read,
                },
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(2),
                    constant: tune_ir::ConstId(2),
                    shape: tune_shape::Shape::Size,
                },
                tune_ir::IrOp::SeqGet {
                    dst: tune_ir::Reg(3),
                    seq: tune_ir::Reg(0),
                    index: tune_ir::Reg(2),
                    checked: true,
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let report = tune_opt::optimize(&mut function);
    assert!(
        report
            .passes
            .iter()
            .any(|pass| { pass.pass == tune_opt::Pass::BoundsCheckElim && pass.changed })
    );
    assert!(matches!(
        function.blocks[0].ops.last(),
        Some(tune_ir::IrOp::SeqGet { checked: false, .. })
    ));
}

#[test]
fn optimizer_eliminates_sequence_bounds_checks_across_blocks_for_stable_locals() {
    let mut function = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        type_params: Vec::new(),
        span: None,
        regs: 4,
        locals: 1,
        constants: vec![
            tune_ir::IrConst::Int(1),
            tune_ir::IrConst::Int(2),
            tune_ir::IrConst::Size(1),
        ],
        struct_layouts: Vec::new(),
        blocks: vec![
            tune_ir::IrBlock {
                id: tune_ir::BlockId(0),
                ops: vec![
                    tune_ir::IrOp::SeqBuild {
                        dst: tune_ir::Reg(0),
                        element_shape: tune_shape::Shape::Int,
                    },
                    tune_ir::IrOp::LoadConst {
                        dst: tune_ir::Reg(1),
                        constant: tune_ir::ConstId(0),
                        shape: tune_shape::Shape::Int,
                    },
                    tune_ir::IrOp::SeqPush {
                        seq: tune_ir::Reg(0),
                        value: tune_ir::Reg(1),
                    },
                    tune_ir::IrOp::LoadConst {
                        dst: tune_ir::Reg(1),
                        constant: tune_ir::ConstId(1),
                        shape: tune_shape::Shape::Int,
                    },
                    tune_ir::IrOp::SeqPush {
                        seq: tune_ir::Reg(0),
                        value: tune_ir::Reg(1),
                    },
                    tune_ir::IrOp::StoreLocal {
                        local: tune_ir::LocalId(0),
                        value: tune_ir::Reg(0),
                        store: tune_ir::IrLocalStore::Init,
                    },
                    tune_ir::IrOp::Jump {
                        target: tune_ir::BlockId(1),
                    },
                ],
            },
            tune_ir::IrBlock {
                id: tune_ir::BlockId(1),
                ops: vec![
                    tune_ir::IrOp::LoadLocal {
                        dst: tune_ir::Reg(0),
                        local: tune_ir::LocalId(0),
                        access: tune_ir::IrLocalAccess::Read,
                    },
                    tune_ir::IrOp::LoadConst {
                        dst: tune_ir::Reg(2),
                        constant: tune_ir::ConstId(2),
                        shape: tune_shape::Shape::Size,
                    },
                    tune_ir::IrOp::SeqGet {
                        dst: tune_ir::Reg(3),
                        seq: tune_ir::Reg(0),
                        index: tune_ir::Reg(2),
                        checked: true,
                    },
                ],
            },
        ],
        task_functions: Vec::new(),
    };

    let report = tune_opt::optimize(&mut function);
    assert!(
        report
            .passes
            .iter()
            .any(|pass| { pass.pass == tune_opt::Pass::BoundsCheckElim && pass.changed })
    );
    assert!(matches!(
        function.blocks[1].ops.last(),
        Some(tune_ir::IrOp::SeqGet { checked: false, .. })
    ));
}

#[test]
fn optimizer_keeps_unproven_sequence_bounds_checks() {
    let mut function = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        type_params: Vec::new(),
        span: None,
        regs: 3,
        locals: 0,
        constants: vec![tune_ir::IrConst::Size(3)],
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::SeqBuild {
                    dst: tune_ir::Reg(0),
                    element_shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(1),
                    constant: tune_ir::ConstId(0),
                    shape: tune_shape::Shape::Size,
                },
                tune_ir::IrOp::SeqGet {
                    dst: tune_ir::Reg(2),
                    seq: tune_ir::Reg(0),
                    index: tune_ir::Reg(1),
                    checked: true,
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let report = tune_opt::optimize(&mut function);
    assert!(
        report
            .passes
            .iter()
            .any(|pass| { pass.pass == tune_opt::Pass::BoundsCheckElim && !pass.changed })
    );
    assert!(matches!(
        function.blocks[0].ops.last(),
        Some(tune_ir::IrOp::SeqGet { checked: true, .. })
    ));
}
