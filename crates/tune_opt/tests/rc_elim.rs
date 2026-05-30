fn sequence_mutation_function(alias_before_set: bool) -> tune_ir::IrFunction {
    let mut ops = vec![
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
            mode: tune_ir::IrMutationMode::Exclusive,
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
    ];
    if alias_before_set {
        ops.push(tune_ir::IrOp::Move {
            dst: tune_ir::Reg(4),
            src: tune_ir::Reg(0),
            transfer: tune_ir::IrTransfer::Copy,
        });
    }
    ops.extend([
        tune_ir::IrOp::LoadConst {
            dst: tune_ir::Reg(2),
            constant: tune_ir::ConstId(2),
            shape: tune_shape::Shape::Size,
        },
        tune_ir::IrOp::LoadConst {
            dst: tune_ir::Reg(3),
            constant: tune_ir::ConstId(1),
            shape: tune_shape::Shape::Int,
        },
        tune_ir::IrOp::SeqSet {
            seq: tune_ir::Reg(0),
            index: tune_ir::Reg(2),
            value: tune_ir::Reg(3),
            checked: false,
            mode: tune_ir::IrMutationMode::SharedCow,
        },
        tune_ir::IrOp::StoreLocal {
            local: tune_ir::LocalId(0),
            value: tune_ir::Reg(0),
            store: tune_ir::IrLocalStore::Assign,
        },
    ]);
    tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        type_params: Vec::new(),
        span: None,
        regs: 5,
        locals: 1,
        constants: vec![
            tune_ir::IrConst::Int(1),
            tune_ir::IrConst::Int(2),
            tune_ir::IrConst::Size(0),
        ],
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops,
        }],
        task_functions: Vec::new(),
    }
}

#[test]
fn optimizer_upgrades_unaliased_sequence_mutation_to_exclusive() {
    let mut function = sequence_mutation_function(false);

    let report = tune_opt::optimize(&mut function);
    assert!(
        report
            .passes
            .iter()
            .any(|pass| { pass.pass == tune_opt::Pass::RcElim && pass.changed })
    );
    assert!(function.blocks[0].ops.iter().any(|op| {
        matches!(
            op,
            tune_ir::IrOp::SeqSet {
                mode: tune_ir::IrMutationMode::Exclusive,
                ..
            }
        )
    }));
}

#[test]
fn optimizer_keeps_sequence_mutation_shared_after_alias() {
    let mut function = sequence_mutation_function(true);

    let report = tune_opt::optimize(&mut function);
    assert!(
        report
            .passes
            .iter()
            .any(|pass| { pass.pass == tune_opt::Pass::RcElim && !pass.changed })
    );
    assert!(function.blocks[0].ops.iter().any(|op| {
        matches!(
            op,
            tune_ir::IrOp::SeqSet {
                mode: tune_ir::IrMutationMode::SharedCow,
                ..
            }
        )
    }));
}
