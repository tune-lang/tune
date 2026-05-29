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
                param_count: 0,
                name: "main".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                register_count: 0,
                local_count: 0,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 0, 0),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
                instructions: Vec::new(),
            },
            tune_bytecode::function::BytecodeFunction {
                param_count: 0,
                name: "<entry>".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                register_count: 3,
                local_count: 0,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 3, 0),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
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
                param_count: 0,
                name: "<entry>".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                register_count: 3,
                local_count: 0,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 3, 0),
                call_sites: vec![tune_bytecode::function::BytecodeCallSite {
                    function: 1,
                    args: vec![0, 1],
                    type_args: Vec::new(),
                }],
                bound_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
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
                param_count: 2,
                name: "add".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                register_count: 3,
                local_count: 2,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(2, 3, 2),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
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

#[test]
fn vm_rejects_too_few_call_arguments() {
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        functions: vec![
            tune_bytecode::function::BytecodeFunction {
                param_count: 0,
                name: "<entry>".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                register_count: 1,
                local_count: 0,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 1, 0),
                call_sites: vec![tune_bytecode::function::BytecodeCallSite {
                    function: 1,
                    args: Vec::new(),
                    type_args: Vec::new(),
                }],
                bound_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
                instructions: vec![tune_bytecode::function::Instruction {
                    opcode: tune_bytecode::Opcode::CallDirect,
                    a: 0,
                    b: 0,
                    c: 0,
                }],
            },
            tune_bytecode::function::BytecodeFunction {
                param_count: 1,
                name: "id".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                register_count: 1,
                local_count: 1,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(1, 1, 1),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
                instructions: vec![tune_bytecode::function::Instruction {
                    opcode: tune_bytecode::Opcode::Return,
                    a: 0,
                    b: 1,
                    c: 0,
                }],
            },
        ],
    };

    let mut vm = tune_vm::Vm::new(artifact);
    assert_eq!(
        vm.run_entry(),
        Err(tune_vm::VmFault::new(
            tune_vm::VmError::InvalidBytecode(
                tune_bytecode::BytecodeValidationError::CallArityMismatch {
                    function: 0,
                    target: 1,
                    expected: 1,
                    actual: 0,
                }
            ),
            None,
        ))
    );
}

#[test]
fn vm_rejects_unsupported_struct_state_plan() {
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        functions: vec![tune_bytecode::function::BytecodeFunction {
            param_count: 0,
            name: "<entry>".into(),
            provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
            register_count: 1,
            local_count: 0,
            frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 1, 0),
            call_sites: Vec::new(),
            bound_call_sites: Vec::new(),
            callable_sites: Vec::new(),
            task_sites: Vec::new(),
            struct_sites: vec![tune_bytecode::function::BytecodeStructSite {
                owner: 0,
                state: tune_bytecode::function::BytecodeStructState {
                    repr: tune_bytecode::function::BytecodeStateRepr::Inline,
                    ownership: tune_bytecode::function::BytecodeOwnershipPlan::SharedAtomic,
                },
                fields: Vec::new(),
            }],
            field_sites: Vec::new(),
            variant_sites: Vec::new(),
            match_sites: Vec::new(),
            for_sites: Vec::new(),
            panic_sites: Vec::new(),
            tuple_sites: Vec::new(),
            string_sites: Vec::new(),
            instructions: vec![tune_bytecode::function::Instruction {
                opcode: tune_bytecode::Opcode::StructConstruct,
                a: 0,
                b: 0,
                c: 0,
            }],
        }],
    };

    let mut vm = tune_vm::Vm::new(artifact);
    assert_eq!(
        vm.run_entry(),
        Err(tune_vm::VmFault::new(
            tune_vm::VmError::UnsupportedStructState,
            Some(tune_vm::VmLocation {
                function: 0,
                function_name: Some("<entry>".to_owned()),
                instruction: Some(0),
                span: None,
            }),
        ))
    );
}

#[test]
fn vm_faults_carry_instruction_span_when_available() {
    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(4),
        tune_diagnostics::ByteOffset::new(9),
    );
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: vec![
            tune_bytecode::artifact::BytecodeConst::Bool(true),
            tune_bytecode::artifact::BytecodeConst::Bool(false),
        ],
        functions: vec![tune_bytecode::function::BytecodeFunction {
            param_count: 0,
            name: "<entry>".into(),
            provenance: tune_bytecode::BytecodeFunctionProvenance {
                span: None,
                instruction_spans: vec![None, None, Some(span)],
            },
            register_count: 3,
            local_count: 0,
            frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 3, 0),
            call_sites: Vec::new(),
            bound_call_sites: Vec::new(),
            callable_sites: Vec::new(),
            task_sites: Vec::new(),
            struct_sites: Vec::new(),
            field_sites: Vec::new(),
            variant_sites: Vec::new(),
            match_sites: Vec::new(),
            for_sites: Vec::new(),
            panic_sites: Vec::new(),
            tuple_sites: Vec::new(),
            string_sites: Vec::new(),
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
            ],
        }],
    };

    let mut vm = tune_vm::Vm::new(artifact);
    assert_eq!(
        vm.run_entry(),
        Err(tune_vm::VmFault::new(
            tune_vm::VmError::UnsupportedOpcode(tune_bytecode::Opcode::AddInt),
            Some(tune_vm::VmLocation {
                function: 0,
                function_name: Some("<entry>".to_owned()),
                instruction: Some(2),
                span: Some(span),
            }),
        ))
    );
}
