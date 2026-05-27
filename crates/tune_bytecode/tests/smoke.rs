#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn core_opcodes_reserve_dense_bytecode_slots() -> Result<(), &'static str> {
    assert_eq!(tune_bytecode::Opcode::ALL.len(), 34);
    for (index, opcode) in tune_bytecode::Opcode::ALL.iter().enumerate() {
        let expected = u8::try_from(index).map_err(|_| "opcode index overflow")?;
        assert_eq!(*opcode as u8, expected);
    }

    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SeqSetChecked));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::VariantConstruct));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::MatchVariant));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::FiniteForInit));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::ResultPropagate));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::SpawnTask));
    assert!(tune_bytecode::Opcode::ALL.contains(&tune_bytecode::Opcode::GreaterInt));

    Ok(())
}

#[test]
fn lowers_typed_local_ir_to_bytecode() -> Result<(), &'static str> {
    let ir = tune_ir::IrFunction {
        owner: None,
        name: "entry".into(),
        regs: 2,
        locals: 1,
        constants: vec![tune_ir::IrConst::Int(1)],
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
    };

    let artifact =
        tune_bytecode::lower_ir_functions(&[ir]).map_err(|_| "ir should lower to bytecode")?;

    assert_eq!(artifact.functions[0].local_count, 1);
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
        owner: None,
        name: "main".into(),
        regs: 3,
        locals: 0,
        constants: vec![tune_ir::IrConst::Int(1), tune_ir::IrConst::Int(2)],
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
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(2)),
                },
            ],
        }],
    };

    let artifact =
        tune_bytecode::lower_ir_functions(&[ir]).map_err(|_| "ir should lower to bytecode")?;

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
fn lowers_direct_call_ir_to_call_site() -> Result<(), &'static str> {
    let entry = tune_ir::IrFunction {
        owner: None,
        name: "<entry>".into(),
        regs: 2,
        locals: 0,
        constants: vec![tune_ir::IrConst::Int(7)],
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
                },
                tune_ir::IrOp::Return {
                    value: Some(tune_ir::Reg(1)),
                },
            ],
        }],
    };
    let callee = tune_ir::IrFunction {
        owner: Some(tune_hir::HirId(1)),
        name: "id".into(),
        regs: 1,
        locals: 1,
        constants: Vec::new(),
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
    };

    let artifact = tune_bytecode::lower_ir_functions(&[entry, callee])
        .map_err(|_| "ir should lower to bytecode")?;

    assert_eq!(artifact.functions[0].call_sites.len(), 1);
    assert_eq!(artifact.functions[0].call_sites[0].function, 1);
    assert_eq!(artifact.functions[0].call_sites[0].args, vec![0]);
    assert_eq!(
        artifact.functions[0].instructions[1].opcode,
        tune_bytecode::Opcode::CallDirect
    );

    Ok(())
}
