#[test]
fn smoke() {
    let crate_name = env!("CARGO_PKG_NAME");
    assert!(!crate_name.is_empty());
}

#[test]
fn vm_executes_integer_add_bytecode_main() -> Result<(), &'static str> {
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        constants: vec!["1".into(), "2".into()],
        functions: vec![tune_bytecode::function::BytecodeFunction {
            name: "main".into(),
            register_count: 3,
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
        }],
    };

    let mut vm = tune_vm::Vm::new(artifact);
    assert_eq!(
        vm.run_main().map_err(|_| "vm should run main")?,
        tune_runtime::value::Value::Int(3)
    );

    Ok(())
}
