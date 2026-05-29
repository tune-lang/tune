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
                span: None,
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
            tune_ir::IrOp::FiniteForNext {
                iterator: tune_ir::Reg(3),
                iterable: tune_ir::Reg(4),
                len: tune_ir::Reg(5),
                index: tune_ir::Reg(6),
                item: tune_ir::Reg(7),
                body: tune_ir::BlockId(1),
                done: tune_ir::BlockId(2),
            },
            tune_ir::IrOp::Spawn {
                dst: tune_ir::Reg(8),
                function: 0,
                captures: Vec::new(),
                span: None,
            },
            tune_ir::IrOp::Return {
                value: Some(tune_ir::Reg(2)),
            },
        ],
    };
    let function = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "run".into(),
        span: None,
        regs: 10,
        locals: 0,
        constants: vec![tune_ir::IrConst::Int(1)],
        blocks: vec![block],
        task_functions: Vec::new(),
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
    let add_span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(4),
        tune_diagnostics::ByteOffset::new(9),
    );
    let plan = tune_plan::PlanFunction {
        name: "main".into(),
        span: None,
        owner: None,
        member: None,
        callable: None,
        params: Vec::new(),
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: vec![
            tune_plan::PlanOp::ConstInt { value: 1 },
            tune_plan::PlanOp::ConstInt { value: 2 },
            tune_plan::PlanOp::BinaryOp {
                op: tune_hir::expr::BinaryOp::Add,
                shape: tune_shape::Shape::Int,
                span: Some(add_span),
            },
            tune_plan::PlanOp::Return,
        ],
    };

    let ir = tune_ir::lower_plan_function(&plan).map_err(|_| "plan should lower")?;

    assert_eq!(ir.regs, 3);
    assert_eq!(ir.locals, 0);
    assert_eq!(
        ir.constants,
        vec![tune_ir::IrConst::Int(1), tune_ir::IrConst::Int(2)]
    );
    assert!(matches!(ir.blocks[0].ops[2], tune_ir::IrOp::AddInt { .. }));
    assert_eq!(ir.blocks[0].ops[2].provenance_span(), Some(add_span));
    assert!(matches!(
        ir.blocks[0].ops[3],
        tune_ir::IrOp::Return { value: Some(_) }
    ));

    Ok(())
}

#[test]
fn lowers_local_binding_plan_to_ir_loads_and_stores() -> Result<(), &'static str> {
    let plan = tune_plan::PlanFunction {
        owner: None,
        member: None,
        callable: None,
        name: "entry".into(),
        span: None,
        params: Vec::new(),
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: vec![
            tune_plan::PlanOp::ConstInt { value: 1 },
            tune_plan::PlanOp::LocalLet {
                local: Some(tune_resolve::LocalId(0)),
                initialized: true,
            },
            tune_plan::PlanOp::BindingGet {
                source: Some(tune_resolve::NameTarget::Local(tune_resolve::LocalId(0))),
            },
            tune_plan::PlanOp::ConstInt { value: 2 },
            tune_plan::PlanOp::BinaryOp {
                op: tune_hir::expr::BinaryOp::Add,
                shape: tune_shape::Shape::Int,
                span: None,
            },
            tune_plan::PlanOp::Return,
        ],
    };

    let ir = tune_ir::lower_plan_function(&plan).map_err(|_| "plan should lower")?;

    assert_eq!(ir.locals, 1);
    assert!(matches!(
        ir.blocks[0].ops[1],
        tune_ir::IrOp::StoreLocal {
            local: tune_resolve::LocalId(0),
            ..
        }
    ));
    assert!(matches!(
        ir.blocks[0].ops[2],
        tune_ir::IrOp::LoadLocal {
            local: tune_resolve::LocalId(0),
            ..
        }
    ));

    Ok(())
}

#[test]
fn lowers_struct_state_plan_to_ir() -> Result<(), &'static str> {
    let field = tune_hir::MemberId {
        owner: tune_hir::HirId(1),
        kind: tune_hir::MemberKind::Field,
        index: 0,
    };
    let plan = tune_plan::PlanFunction {
        owner: None,
        member: None,
        callable: None,
        name: "entry".into(),
        span: None,
        params: Vec::new(),
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: vec![
            tune_plan::PlanOp::ConstInt { value: 1 },
            tune_plan::PlanOp::StructConstruct {
                item: tune_hir::HirId(1),
                escape: tune_plan::StructEscapeReason::Local,
                state: tune_plan::StructStatePlan::LOCAL,
                fields: vec![field],
                span: None,
            },
            tune_plan::PlanOp::Return,
        ],
    };

    let ir = tune_ir::lower_plan_function(&plan).map_err(|_| "plan should lower")?;

    assert!(matches!(
        ir.blocks[0].ops[1],
        tune_ir::IrOp::StructConstruct {
            state: tune_ir::IrStructState {
                repr: tune_ir::IrStateRepr::LocalHandle,
                ownership: tune_ir::IrOwnershipPlan::NonAtomicRc,
            },
            ..
        }
    ));

    Ok(())
}

#[test]
fn lowers_direct_call_plan_to_ir_with_param_slots() -> Result<(), &'static str> {
    let param = tune_hir::MemberId {
        owner: tune_hir::HirId(1),
        kind: tune_hir::MemberKind::Param,
        index: 0,
    };
    let plan = tune_plan::PlanFunction {
        owner: Some(tune_hir::HirId(1)),
        member: None,
        callable: None,
        name: "id".into(),
        span: None,
        params: vec![param],
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: vec![
            tune_plan::PlanOp::BindingGet {
                source: Some(tune_resolve::NameTarget::Param(param)),
            },
            tune_plan::PlanOp::Return,
        ],
    };

    let ir = tune_ir::lower_plan_function(&plan).map_err(|_| "plan should lower")?;

    assert_eq!(ir.owner, Some(tune_hir::HirId(1)));
    assert_eq!(ir.locals, 1);
    assert!(matches!(
        ir.blocks[0].ops[0],
        tune_ir::IrOp::LoadLocal {
            local: tune_resolve::LocalId(0),
            ..
        }
    ));

    let entry = tune_plan::PlanFunction {
        owner: None,
        member: None,
        callable: None,
        name: "<entry>".into(),
        span: None,
        params: Vec::new(),
        local_params: Vec::new(),
        captures: Vec::new(),
        module_bindings: Vec::new(),
        ops: vec![
            tune_plan::PlanOp::ConstInt { value: 7 },
            tune_plan::PlanOp::DirectCall {
                target: tune_hir::HirId(1),
                arg_count: 1,
                type_args: Vec::new(),
                span: None,
            },
            tune_plan::PlanOp::Return,
        ],
    };
    let ir = tune_ir::lower_plan_function(&entry).map_err(|_| "entry should lower")?;

    assert!(matches!(
        ir.blocks[0].ops[1],
        tune_ir::IrOp::CallDirect {
            function: tune_hir::HirId(1),
            ref args,
            ..
        } if args == &vec![tune_ir::Reg(0)]
    ));

    Ok(())
}
