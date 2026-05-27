#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn vm_executes_integer_add_bytecode_entry() -> Result<(), &'static str> {
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(1),
        constants: vec![
            tune_bytecode::artifact::BytecodeConst::Int(1),
            tune_bytecode::artifact::BytecodeConst::Int(2),
        ],
        functions: vec![
            tune_bytecode::function::BytecodeFunction {
                name: "main".into(),
                register_count: 0,
                local_count: 0,
                call_sites: Vec::new(),
                struct_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                instructions: Vec::new(),
            },
            tune_bytecode::function::BytecodeFunction {
                name: "<entry>".into(),
                register_count: 3,
                local_count: 0,
                call_sites: Vec::new(),
                struct_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                instructions: vec![
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::LoadConst,
                        a: 0,
                        b: 0,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::LoadConst,
                        a: 1,
                        b: 1,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::AddInt,
                        a: 2,
                        b: 0,
                        c: 1,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::Return,
                        a: 2,
                        b: 1,
                        c: 0,
                    },
                ],
            },
        ],
    };

    let mut vm = tune_vm::Vm::new(artifact);
    assert_eq!(
        vm.run_entry().map_err(|_| "vm should run entry")?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}

#[test]
fn vm_executes_direct_call_with_arguments() -> Result<(), &'static str> {
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: vec![
            tune_bytecode::artifact::BytecodeConst::Int(1),
            tune_bytecode::artifact::BytecodeConst::Int(2),
        ],
        functions: vec![
            tune_bytecode::function::BytecodeFunction {
                name: "<entry>".into(),
                register_count: 3,
                local_count: 0,
                call_sites: vec![tune_bytecode::function::BytecodeCallSite {
                    function: 1,
                    args: vec![0, 1],
                }],
                struct_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                instructions: vec![
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::LoadConst,
                        a: 0,
                        b: 0,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::LoadConst,
                        a: 1,
                        b: 1,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::CallDirect,
                        a: 2,
                        b: 0,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::Return,
                        a: 2,
                        b: 1,
                        c: 0,
                    },
                ],
            },
            tune_bytecode::function::BytecodeFunction {
                name: "add".into(),
                register_count: 3,
                local_count: 2,
                call_sites: Vec::new(),
                struct_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                instructions: vec![
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::LoadLocal,
                        a: 0,
                        b: 0,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::LoadLocal,
                        a: 1,
                        b: 1,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::AddInt,
                        a: 2,
                        b: 0,
                        c: 1,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::Return,
                        a: 2,
                        b: 1,
                        c: 0,
                    },
                ],
            },
        ],
    };

    let mut vm = tune_vm::Vm::new(artifact);
    assert_eq!(
        vm.run_entry().map_err(|_| "vm should run entry")?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}
