#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn ir_has_typed_slots_for_core_planned_operations() {
    let block = tune_ir::IrBlock {
        id: tune_ir::BlockId(0),
        ops: vec![
            tune_ir::IrOp::LoadConst {
                dst: tune_ir::Reg(0),
                constant: tune_ir::ConstId(0),
                shape: tune_shape::Shape::Int,
            },
            tune_ir::IrOp::VariantConstruct {
                dst: tune_ir::Reg(1),
                variant: tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Ok),
                args: vec![tune_ir::Reg(0)],
            },
            tune_ir::IrOp::ResultPropagate {
                dst: tune_ir::Reg(2),
                result: tune_ir::Reg(1),
                expr: tune_hir::ExprId(0),
                span: None,
            },
            tune_ir::IrOp::FiniteForInit {
                iterator: tune_ir::Reg(3),
                iterable: tune_ir::Reg(4),
                len: tune_ir::Reg(5),
            },
            tune_ir::IrOp::Spawn {
                dst: tune_ir::Reg(6),
                callable: tune_ir::Reg(7),
            },
            tune_ir::IrOp::Return {
                value: Some(tune_ir::Reg(2)),
            },
        ],
    };
    let function = tune_ir::IrFunction {
        name: "run".into(),
        regs: 8,
        constants: vec![tune_ir::IrConst::Int(1)],
        blocks: vec![block],
    };

    assert_eq!(function.blocks.len(), 1);
    assert!(matches!(
        function.blocks[0].ops[1],
        tune_ir::IrOp::VariantConstruct { .. }
    ));
    assert!(matches!(
        function.blocks[0].ops[2],
        tune_ir::IrOp::ResultPropagate { .. }
    ));
}

#[test]
fn lowers_integer_add_plan_to_ir() -> Result<(), &'static str> {
    let plan = tune_plan::PlanFunction {
        name: "main".into(),
        ops: vec![
            tune_plan::PlanOp::ConstInt { value: 1 },
            tune_plan::PlanOp::ConstInt { value: 2 },
            tune_plan::PlanOp::BinaryOp {
                op: tune_hir::expr::BinaryOp::Add,
            },
            tune_plan::PlanOp::Return,
        ],
    };

    let ir = tune_ir::lower_plan_function(&plan).map_err(|_| "plan should lower")?;

    assert_eq!(ir.regs, 3);
    assert_eq!(
        ir.constants,
        vec![tune_ir::IrConst::Int(1), tune_ir::IrConst::Int(2)]
    );
    assert!(matches!(ir.blocks[0].ops[2], tune_ir::IrOp::AddInt { .. }));
    assert!(matches!(
        ir.blocks[0].ops[3],
        tune_ir::IrOp::Return { value: Some(_) }
    ));

    Ok(())
}
