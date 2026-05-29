#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn core_opcodes_reserve_dense_bytecode_slots() -> Result<(), &'static str> {
    assert_eq!(tune_bytecode::Opcode::ALL.len(), 97);
    for (index, opcode) in tune_bytecode::Opcode::ALL.iter().enumerate() {
        let expected = u8::try_from(index).map_err(|_| "opcode index overflow")?;
        assert_eq!(*opcode as u8, expected);
    }

    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SeqSetChecked));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::TupleBuild));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::VariantConstruct));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::StructConstruct));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::MatchVariant));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::FiniteForInit));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::ResultPropagate));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SpawnTask));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::VariantField));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::TupleField));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::EqualInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::NotEqualInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessEqualInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterEqualInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::NegInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::NotBool));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::RangeExclusiveInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::RangeInclusiveInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SubInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::MulInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::DivInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::RemInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitNotInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitAndInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitOrInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitXorInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::ShiftLeftInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::ShiftRightInt));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::NoneCheck));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SubFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::MulFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::DivFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::EqualFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::NotEqualFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessEqualFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterEqualFloat));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SubSizeChecked));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::MulSizeChecked));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::DivSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::RemSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::EqualSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::NotEqualSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessEqualSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterEqualSize));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SubByteWrap));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::MulByteWrap));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::DivByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::RemByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitNotByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitAndByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitOrByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::BitXorByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::ShiftLeftByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::ShiftRightByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::EqualByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::NotEqualByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::LessEqualByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterEqualByte));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::StringLen));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::StringGet));

    Ok(())
}

#[test]
fn lowers_typed_local_ir_to_bytecode() -> Result<(), &'static str> {
    let ir = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "entry".into(),
        type_params: Vec::new(),
        span: None,
        regs: 2,
        locals: 1,
        constants: vec![tune_ir::IrConst::Int(1)],
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(0),
                    constant: tune_ir::ConstId(0),
                    shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::StoreLocal {
                    local: tune_resolve::LocalId(0),
                    value: tune_ir::Reg(0),
                },
                tune_ir::IrOp::LoadLocal {
                    dst: tune_ir::Reg(1),
                    local: tune_resolve::LocalId(0),
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(1)),
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let artifact =
        tune_bytecode::lower_ir_functions(&[ir]).map_err(|_| "ir should lower to bytecode")?;
    tune_bytecode::validate_artifact(&artifact).map_err(|_| "bytecode should validate")?;

    assert_eq!(artifact.functions[0].local_count, 1);
    assert_eq!(
        artifact.functions[0].frame.locals,
        vec![tune_shape::Shape::Int]
    );
    assert_eq!(
        artifact.functions[0].frame.registers,
        vec![tune_shape::Shape::Int, tune_shape::Shape::Int]
    );
    assert_eq!(
        artifact.functions[0].instructions[1].opcode,
        tune_bytecode::Opcode::StoreLocal
    );
    assert_eq!(
        artifact.functions[0].instructions[2].opcode,
        tune_bytecode::Opcode::LoadLocal
    );

    Ok(())
}

#[test]
fn lowers_integer_add_ir_to_bytecode() -> Result<(), &'static str> {
    let ir = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "main".into(),
        type_params: Vec::new(),
        span: None,
        regs: 3,
        locals: 0,
        constants: vec![tune_ir::IrConst::Int(1), tune_ir::IrConst::Int(2)],
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(0),
                    constant: tune_ir::ConstId(0),
                    shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(1),
                    constant: tune_ir::ConstId(1),
                    shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::AddInt {
                    dst: tune_ir::Reg(2),
                    a: tune_ir::Reg(0),
                    b: tune_ir::Reg(1),
                    span: None,
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(2)),
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let artifact =
        tune_bytecode::lower_ir_functions(&[ir]).map_err(|_| "ir should lower to bytecode")?;
    tune_bytecode::validate_artifact(&artifact).map_err(|_| "bytecode should validate")?;

    assert_eq!(
        artifact.constants,
        vec![
            tune_bytecode::artifact::BytecodeConst::Int(1),
            tune_bytecode::artifact::BytecodeConst::Int(2),
        ]
    );
    assert_eq!(artifact.functions[0].instructions.len(), 4);
    assert_eq!(
        artifact.functions[0].instructions[2].opcode,
        tune_bytecode::Opcode::AddInt
    );

    Ok(())
}

#[test]
fn lowers_struct_construct_with_explicit_local_state_plan() -> Result<(), &'static str> {
    let ir = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "entry".into(),
        type_params: Vec::new(),
        span: None,
        regs: 2,
        locals: 0,
        constants: vec![tune_ir::IrConst::Int(1)],
        struct_layouts: vec![tune_ir::IrStructLayout {
            owner: tune_hir::HirId(7),
            fields: vec![tune_ir::FieldId(0)],
        }],
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(0),
                    constant: tune_ir::ConstId(0),
                    shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::StructConstruct {
                    dst: tune_ir::Reg(1),
                    item: tune_hir::HirId(7),
                    state: tune_ir::IrStructState {
                        repr: tune_ir::IrStateRepr::LocalHandle,
                        ownership: tune_ir::IrOwnershipPlan::NonAtomicRc,
                    },
                    fields: vec![tune_ir::StructField {
                        field: tune_ir::FieldId(0),
                        value: tune_ir::Reg(0),
                    }],
                    span: None,
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(1)),
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let artifact =
        tune_bytecode::lower_ir_functions(&[ir]).map_err(|_| "ir should lower to bytecode")?;
    tune_bytecode::validate_artifact(&artifact).map_err(|_| "bytecode should validate")?;
    let site = &artifact.functions[0].struct_sites[0];

    assert_eq!(site.owner, 7);
    assert_eq!(
        site.state,
        tune_bytecode::function::BytecodeStructState::LOCAL
    );
    Ok(())
}

#[test]
fn lowers_direct_call_ir_to_call_site() -> Result<(), &'static str> {
    let call_span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(2),
        tune_diagnostics::ByteOffset::new(12),
        tune_diagnostics::ByteOffset::new(18),
    );
    let entry = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "<entry>".into(),
        type_params: Vec::new(),
        span: None,
        regs: 2,
        locals: 0,
        constants: vec![tune_ir::IrConst::Int(7)],
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(0),
                    constant: tune_ir::ConstId(0),
                    shape: tune_shape::Shape::Int,
                },
                tune_ir::IrOp::CallDirect {
                    dst: tune_ir::Reg(1),
                    function: tune_hir::HirId(1),
                    args: vec![tune_ir::Reg(0)],
                    type_args: vec![tune_shape::Shape::Int],
                    span: Some(call_span),
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(1)),
                },
            ],
        }],
        task_functions: Vec::new(),
    };
    let callee = tune_ir::IrFunction {
        params: 1,
        owner: Some(tune_hir::HirId(1)),
        member: None,
        callable: None,
        name: "id".into(),
        type_params: vec!["T".into()],
        span: None,
        regs: 1,
        locals: 1,
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::LoadLocal {
                    dst: tune_ir::Reg(0),
                    local: tune_resolve::LocalId(0),
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(0)),
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let artifact = tune_bytecode::lower_ir_functions(&[entry, callee])
        .map_err(|_| "ir should lower to bytecode")?;
    tune_bytecode::validate_artifact(&artifact).map_err(|_| "bytecode should validate")?;

    assert_eq!(artifact.functions[0].call_sites.len(), 1);
    assert_eq!(artifact.functions[0].call_sites[0].function, 1);
    assert_eq!(artifact.functions[0].call_sites[0].args, vec![0]);
    assert_eq!(
        artifact.functions[0].call_sites[0].type_args,
        vec![tune_shape::Shape::Int]
    );
    assert_eq!(
        artifact.functions[0].instructions[1].opcode,
        tune_bytecode::Opcode::CallDirect
    );
    assert_eq!(artifact.instruction_span(0, 1), Some(call_span));

    Ok(())
}

#[test]
fn lowers_host_call_ir_to_host_call_site() -> Result<(), &'static str> {
    let ir = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "<entry>".into(),
        type_params: Vec::new(),
        span: None,
        regs: 2,
        locals: 0,
        constants: vec![tune_ir::IrConst::String("42".into())],
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::LoadConst {
                    dst: tune_ir::Reg(0),
                    constant: tune_ir::ConstId(0),
                    shape: tune_shape::Shape::String,
                },
                tune_ir::IrOp::CallHost {
                    dst: tune_ir::Reg(1),
                    symbol: tune_ir::HostSymbolId(3),
                    task_safe: true,
                    args: vec![tune_ir::Reg(0)],
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(1)),
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let artifact =
        tune_bytecode::lower_ir_functions(&[ir]).map_err(|_| "ir should lower to bytecode")?;
    tune_bytecode::validate_artifact(&artifact).map_err(|_| "bytecode should validate")?;

    assert_eq!(artifact.functions[0].host_call_sites.len(), 1);
    assert_eq!(
        artifact.functions[0].host_call_sites[0].symbol,
        tune_host::HostSymbolId(3)
    );
    assert_eq!(artifact.functions[0].host_call_sites[0].args, vec![0]);
    assert_eq!(
        artifact.functions[0].instructions[1].opcode,
        tune_bytecode::Opcode::CallHost
    );

    Ok(())
}

#[test]
fn lowering_preserves_function_and_instruction_provenance() -> Result<(), &'static str> {
    let function_span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(10),
        tune_diagnostics::ByteOffset::new(30),
    );
    let propagate_span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(20),
        tune_diagnostics::ByteOffset::new(21),
    );
    let ir = tune_ir::IrFunction {
        params: 0,
        owner: None,
        member: None,
        callable: None,
        name: "entry".into(),
        type_params: Vec::new(),
        span: Some(function_span),
        regs: 2,
        locals: 0,
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        blocks: vec![tune_ir::IrBlock {
            id: tune_ir::BlockId(0),
            ops: vec![
                tune_ir::IrOp::ResultPropagate {
                    dst: tune_ir::Reg(1),
                    result: tune_ir::Reg(0),
                    expr: tune_hir::ExprId(7),
                    span: Some(propagate_span),
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(1)),
                },
            ],
        }],
        task_functions: Vec::new(),
    };

    let artifact =
        tune_bytecode::lower_ir_functions(&[ir]).map_err(|_| "ir should lower to bytecode")?;
    tune_bytecode::validate_artifact(&artifact).map_err(|_| "bytecode should validate")?;

    assert_eq!(artifact.function_span(0), Some(function_span));
    assert_eq!(artifact.instruction_span(0, 0), Some(propagate_span));
    assert_eq!(artifact.instruction_span(0, 1), Some(function_span));

    Ok(())
}
