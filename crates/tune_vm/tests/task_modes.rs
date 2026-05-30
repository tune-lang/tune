#[test]
fn vm_executes_task_in_eager_mode_and_propagates_panic_at_spawn() {
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: vec![
            tune_bytecode::artifact::BytecodeConst::String("spawned panic".into()),
            tune_bytecode::artifact::BytecodeConst::Int(7),
        ],
        struct_layouts: Vec::new(),
        functions: vec![
            tune_bytecode::function::BytecodeFunction {
                param_count: 0,
                name: "<entry>".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                generic_param_count: 0,
                register_count: 2,
                local_count: 0,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 2, 0),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                host_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: vec![tune_bytecode::function::BytecodeTaskSite {
                    function: 1,
                    captures: Vec::new(),
                }],
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
                        opcode: tune_bytecode::Opcode::SpawnTask,
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
                        opcode: tune_bytecode::Opcode::Return,
                        a: 1,
                        b: 1,
                        c: 0,
                    },
                ],
            },
            tune_bytecode::function::BytecodeFunction {
                param_count: 0,
                name: "spawned_task".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                generic_param_count: 0,
                register_count: 1,
                local_count: 1,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 1, 1),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                host_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: Vec::new(),
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: vec![tune_bytecode::function::BytecodePanicSite { args: vec![0] }],
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
                        opcode: tune_bytecode::Opcode::Panic,
                        a: 0,
                        b: 0,
                        c: 0,
                    },
                ],
            },
        ],
    };

    let mut parallel = tune_vm::Vm::new(artifact.clone());
    assert_eq!(
        parallel.run_entry(),
        Ok(tune_runtime::Value::Int(7)),
        "unjoined parallel task failures should not surface in the parent frame",
    );

    let mut immediate =
        tune_vm::Vm::new(artifact).with_task_execution(tune_runtime::TaskExecutionMode::Immediate);
    assert!(matches!(
        immediate.run_entry(),
        Err(tune_vm::VmFault {
            error: tune_vm::VmError::Panic(_),
            ..
        })
    ));
}

#[test]
fn vm_rejects_task_unsafe_capture_in_eager_mode_at_spawn() {
    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: vec![tune_bytecode::artifact::BytecodeConst::String(
            "marker".into(),
        )],
        struct_layouts: Vec::new(),
        functions: vec![
            tune_bytecode::function::BytecodeFunction {
                param_count: 0,
                name: "<entry>".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                generic_param_count: 0,
                register_count: 3,
                local_count: 0,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 3, 0),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                host_call_sites: vec![tune_bytecode::function::BytecodeHostCallSite {
                    symbol: tune_host::HostSymbolId(0),
                    task_safe: true,
                    args: vec![0],
                }],
                callable_sites: Vec::new(),
                task_sites: vec![tune_bytecode::function::BytecodeTaskSite {
                    function: 1,
                    captures: vec![tune_bytecode::function::BytecodeCapture {
                        register: 1,
                        mode: tune_bytecode::function::BytecodeCaptureMode::Reference,
                    }],
                }],
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
                        opcode: tune_bytecode::Opcode::CallHost,
                        a: 1,
                        b: 0,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::SpawnTask,
                        a: 2,
                        b: 0,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::LoadConst,
                        a: 0,
                        b: 0,
                        c: 0,
                    },
                    tune_bytecode::function::Instruction {
                        opcode: tune_bytecode::Opcode::Return,
                        a: 2,
                        b: 2,
                        c: 0,
                    },
                ],
            },
            tune_bytecode::function::BytecodeFunction {
                param_count: 1,
                name: "spawned_task".into(),
                provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
                generic_param_count: 0,
                register_count: 1,
                local_count: 1,
                frame: tune_bytecode::function::BytecodeFrameLayout::unknown(1, 1, 1),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                host_call_sites: Vec::new(),
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
                        opcode: tune_bytecode::Opcode::Return,
                        a: 0,
                        b: 0,
                        c: 0,
                    },
                ],
            },
        ],
    };

    let mut immediate =
        tune_vm::Vm::new(artifact).with_task_execution(tune_runtime::TaskExecutionMode::Immediate);
    let executor = tune_host::HostExecutor::new(|args: &[tune_runtime::Value]| {
        let Some(tune_runtime::Value::String(label)) = args.first() else {
            return Err(tune_host::HostCallError::new("expected string"));
        };
        assert_eq!(label, "marker");
        Ok(tune_runtime::Value::Resource(
            tune_runtime::ResourceHandle::new(tune_runtime::ResourceId(1), "fs.File"),
        ))
    });
    immediate = immediate.with_host_executors(vec![executor]);

    let result = immediate.run_entry();
    assert!(
        matches!(
            result,
            Err(tune_vm::VmFault {
                error: tune_vm::VmError::TaskUnsafeCapture { .. },
                ..
            })
        ),
        "{result:?}"
    );
}

#[test]
fn vm_spawned_functions_share_struct_receiver_state() {
    use tune_bytecode::artifact::{BytecodeArtifact, BytecodeConst};
    use tune_bytecode::function::{
        BytecodeCapture, BytecodeCaptureMode, BytecodeFieldSite, BytecodeFrameLayout,
        BytecodeFunction, BytecodeOwnershipPlan, BytecodeStateRepr, BytecodeStructField,
        BytecodeStructLayout, BytecodeStructSite, BytecodeStructState, BytecodeTaskSite,
        Instruction,
    };
    use tune_bytecode::{BytecodeFunctionProvenance, Opcode};

    let artifact = BytecodeArtifact {
        entry_function: Some(0),
        constants: vec![BytecodeConst::Int(0), BytecodeConst::Int(1)],
        struct_layouts: vec![BytecodeStructLayout {
            owner: 0,
            fields: vec![0],
        }],
        functions: vec![
            BytecodeFunction {
                param_count: 0,
                name: "<entry>".into(),
                provenance: BytecodeFunctionProvenance::default(),
                generic_param_count: 0,
                register_count: 7,
                local_count: 0,
                frame: BytecodeFrameLayout::unknown(0, 7, 0),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                host_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: vec![
                    BytecodeTaskSite {
                        function: 1,
                        captures: vec![BytecodeCapture {
                            register: 1,
                            mode: BytecodeCaptureMode::Reference,
                        }],
                    },
                    BytecodeTaskSite {
                        function: 1,
                        captures: vec![BytecodeCapture {
                            register: 1,
                            mode: BytecodeCaptureMode::Reference,
                        }],
                    },
                ],
                struct_sites: vec![BytecodeStructSite {
                    owner: 0,
                    state: BytecodeStructState {
                        repr: BytecodeStateRepr::SharedHandle,
                        ownership: BytecodeOwnershipPlan::SharedAtomic,
                    },
                    fields: vec![BytecodeStructField { field: 0, value: 0 }],
                }],
                field_sites: vec![BytecodeFieldSite { owner: 0, field: 0 }],
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
                instructions: vec![
                    inst(Opcode::LoadConst, 0, 0, 0),
                    inst(Opcode::StructConstruct, 1, 0, 0),
                    inst(Opcode::SpawnTask, 2, 0, 0),
                    inst(Opcode::SpawnTask, 3, 1, 0),
                    inst(Opcode::TaskJoin, 4, 2, 0),
                    inst(Opcode::TaskJoin, 5, 3, 0),
                    inst(Opcode::FieldGet, 6, 1, 0),
                    inst(Opcode::Return, 6, 6, 0),
                ],
            },
            BytecodeFunction {
                param_count: 1,
                name: "bump_task".into(),
                provenance: BytecodeFunctionProvenance::default(),
                generic_param_count: 0,
                register_count: 5,
                local_count: 1,
                frame: BytecodeFrameLayout::unknown(1, 5, 1),
                call_sites: Vec::new(),
                bound_call_sites: Vec::new(),
                host_call_sites: Vec::new(),
                callable_sites: Vec::new(),
                task_sites: Vec::new(),
                struct_sites: Vec::new(),
                field_sites: vec![BytecodeFieldSite { owner: 0, field: 0 }],
                variant_sites: Vec::new(),
                match_sites: Vec::new(),
                for_sites: Vec::new(),
                panic_sites: Vec::new(),
                tuple_sites: Vec::new(),
                string_sites: Vec::new(),
                instructions: vec![
                    inst(Opcode::LoadLocal, 0, 0, 0),
                    inst(Opcode::FieldGet, 1, 0, 0),
                    inst(Opcode::LoadConst, 2, 1, 0),
                    inst(Opcode::AddInt, 3, 1, 2),
                    inst(Opcode::FieldSet, 0, 0, 3),
                    inst(Opcode::FieldGet, 4, 0, 0),
                    inst(Opcode::Return, 4, 4, 0),
                ],
            },
        ],
    };

    let mut vm = tune_vm::Vm::new(artifact)
        .with_task_execution(tune_runtime::TaskExecutionMode::DeferredUntilJoin);

    assert_eq!(vm.run_entry(), Ok(tune_runtime::Value::Int(2)));

    fn inst(opcode: Opcode, a: u32, b: u32, c: u32) -> Instruction {
        Instruction { opcode, a, b, c }
    }
}
