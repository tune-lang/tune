fn empty_function(
    name: &str,
    registers: u32,
    locals: u32,
) -> tune_bytecode::function::BytecodeFunction {
    tune_bytecode::function::BytecodeFunction {
        param_count: 0,
        name: name.into(),
        provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
        generic_param_count: 0,
        register_count: registers,
        local_count: locals,
        frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, registers, locals),
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
        instructions: Vec::new(),
    }
}

#[test]
fn rejects_call_arity_mismatch() {
    let mut entry = empty_function("<entry>", 1, 0);
    entry
        .call_sites
        .push(tune_bytecode::function::BytecodeCallSite {
            function: 1,
            args: Vec::new(),
            type_args: vec![tune_shape::Shape::Int],
        });
    entry
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::CallDirect,
            a: 0,
            b: 0,
            c: 0,
        });

    let mut callee = empty_function("id", 1, 1);
    callee.param_count = 1;
    callee.frame = tune_bytecode::function::BytecodeFrameLayout::unknown(1, 1, 1);
    callee
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::Return,
            a: 0,
            b: 1,
            c: 0,
        });

    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        functions: vec![entry, callee],
    };

    assert_eq!(
        tune_bytecode::validate_artifact(&artifact),
        Err(tune_bytecode::BytecodeValidationError::CallArityMismatch {
            function: 0,
            target: 1,
            expected: 1,
            actual: 0,
        })
    );
}

#[test]
fn rejects_generic_arg_arity_mismatch() {
    let mut entry = empty_function("<entry>", 1, 0);
    entry
        .call_sites
        .push(tune_bytecode::function::BytecodeCallSite {
            function: 1,
            args: Vec::new(),
            type_args: vec![tune_shape::Shape::Int],
        });
    entry
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::CallDirect,
            a: 0,
            b: 0,
            c: 0,
        });

    let mut callee = empty_function("id", 1, 0);
    callee.generic_param_count = 2;
    callee
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::Return,
            a: 0,
            b: 0,
            c: 0,
        });

    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        functions: vec![entry, callee],
    };

    assert_eq!(
        tune_bytecode::validate_artifact(&artifact),
        Err(
            tune_bytecode::BytecodeValidationError::GenericArgArityMismatch {
                function: 0,
                target: 1,
                expected: 2,
                actual: 1,
            }
        )
    );
}

#[test]
fn rejects_register_out_of_bounds() {
    let mut function = empty_function("<entry>", 1, 0);
    function
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::LoadConst,
            a: 1,
            b: 0,
            c: 0,
        });

    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: vec![tune_bytecode::artifact::BytecodeConst::Int(1)],
        struct_layouts: Vec::new(),
        functions: vec![function],
    };

    assert_eq!(
        tune_bytecode::validate_artifact(&artifact),
        Err(
            tune_bytecode::BytecodeValidationError::RegisterOutOfBounds {
                function: 0,
                register: 1,
            }
        )
    );
}

#[test]
fn rejects_unknown_field_index() {
    let mut function = empty_function("<entry>", 2, 0);
    function
        .struct_sites
        .push(tune_bytecode::function::BytecodeStructSite {
            owner: 0,
            state: tune_bytecode::function::BytecodeStructState::LOCAL,
            fields: vec![tune_bytecode::function::BytecodeStructField { field: 0, value: 0 }],
        });
    function
        .field_sites
        .push(tune_bytecode::function::BytecodeFieldSite { owner: 0, field: 1 });
    function
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::FieldGet,
            a: 0,
            b: 1,
            c: 0,
        });

    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        struct_layouts: vec![tune_bytecode::function::BytecodeStructLayout {
            owner: 0,
            fields: vec![0],
        }],
        functions: vec![function],
    };

    assert_eq!(
        tune_bytecode::validate_artifact(&artifact),
        Err(
            tune_bytecode::BytecodeValidationError::FieldIndexOutOfBounds {
                function: 0,
                field: 1,
            }
        )
    );
}

#[test]
fn rejects_unknown_struct_layout() {
    let mut function = empty_function("<entry>", 2, 0);
    function
        .struct_sites
        .push(tune_bytecode::function::BytecodeStructSite {
            owner: 9,
            state: tune_bytecode::function::BytecodeStructState::LOCAL,
            fields: Vec::new(),
        });
    function
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::StructConstruct,
            a: 0,
            b: 0,
            c: 0,
        });

    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        functions: vec![function],
    };

    assert_eq!(
        tune_bytecode::validate_artifact(&artifact),
        Err(
            tune_bytecode::BytecodeValidationError::StructLayoutMissing {
                function: 0,
                owner: 9,
            }
        )
    );
}

#[test]
fn accepts_field_access_from_declared_struct_layout() {
    let mut function = empty_function("<entry>", 2, 0);
    function
        .field_sites
        .push(tune_bytecode::function::BytecodeFieldSite { owner: 7, field: 3 });
    function
        .instructions
        .push(tune_bytecode::function::Instruction {
            opcode: tune_bytecode::Opcode::FieldGet,
            a: 0,
            b: 0,
            c: 0,
        });

    let artifact = tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        struct_layouts: vec![tune_bytecode::function::BytecodeStructLayout {
            owner: 7,
            fields: vec![3],
        }],
        functions: vec![function],
    };

    assert_eq!(tune_bytecode::validate_artifact(&artifact), Ok(()));
}
