fn host_resource_artifact() -> tune_bytecode::artifact::BytecodeArtifact {
    tune_bytecode::artifact::BytecodeArtifact {
        entry_function: Some(0),
        constants: Vec::new(),
        struct_layouts: Vec::new(),
        functions: vec![tune_bytecode::function::BytecodeFunction {
            param_count: 0,
            name: "<entry>".into(),
            provenance: tune_bytecode::BytecodeFunctionProvenance::default(),
            generic_param_count: 0,
            register_count: 1,
            local_count: 0,
            frame: tune_bytecode::function::BytecodeFrameLayout::unknown(0, 1, 0),
            call_sites: Vec::new(),
            bound_call_sites: Vec::new(),
            host_call_sites: vec![tune_bytecode::function::BytecodeHostCallSite {
                symbol: tune_host::HostSymbolId(0),
                task_safe: true,
                args: Vec::new(),
            }],
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
                    opcode: tune_bytecode::Opcode::CallHost,
                    a: 0,
                    b: 0,
                    c: 0,
                },
                tune_bytecode::function::Instruction {
                    opcode: tune_bytecode::Opcode::Return,
                    a: 0,
                    b: 1,
                    c: 0,
                },
            ],
        }],
    }
}

#[test]
fn vm_canonicalizes_registered_host_resource_metadata() -> Result<(), &'static str> {
    let executor = tune_host::HostExecutor::new(|_: &[tune_runtime::Value]| {
        Ok(tune_runtime::Value::Resource(
            tune_runtime::ResourceHandle::new(tune_runtime::ResourceId(7), "fs.File")
                .task_safe(true),
        ))
    });
    let mut vm = tune_vm::Vm::new(host_resource_artifact())
        .with_host_executors(vec![executor])
        .with_host_resource_types(vec![tune_vm::VmHostResourceType::new(
            tune_runtime::ResourceTypeId(0),
            "fs.File",
        )]);

    let tune_runtime::Value::Resource(resource) =
        vm.run_entry().map_err(|_| "vm should run entry")?
    else {
        return Err("expected resource");
    };
    assert_eq!(resource.id, tune_runtime::ResourceId(7));
    assert_eq!(resource.type_id, Some(tune_runtime::ResourceTypeId(0)));
    assert_eq!(resource.type_name, "fs.File");
    assert!(!resource.task_safe);

    Ok(())
}

#[test]
fn vm_rejects_unknown_host_resource_when_registry_is_installed() {
    let executor = tune_host::HostExecutor::new(|_: &[tune_runtime::Value]| {
        Ok(tune_runtime::Value::Resource(
            tune_runtime::ResourceHandle::new(tune_runtime::ResourceId(7), "net.Socket"),
        ))
    });
    let mut vm = tune_vm::Vm::new(host_resource_artifact())
        .with_host_executors(vec![executor])
        .with_host_resource_types(vec![tune_vm::VmHostResourceType::new(
            tune_runtime::ResourceTypeId(0),
            "fs.File",
        )]);

    assert!(matches!(
        vm.run_entry(),
        Err(tune_vm::VmFault {
            error: tune_vm::VmError::UnknownHostResourceType { .. },
            ..
        })
    ));
}

#[test]
fn vm_enforces_registered_resource_authorities() {
    let executor = tune_host::HostExecutor::new(|_: &[tune_runtime::Value]| {
        Ok(tune_runtime::Value::Resource(
            tune_runtime::ResourceHandle::new(tune_runtime::ResourceId(7), "fs.File"),
        ))
    });
    let resource_type =
        tune_vm::VmHostResourceType::new(tune_runtime::ResourceTypeId(0), "fs.File")
            .with_authorities(vec![tune_host::Authority("fs.read".into())]);
    let mut vm = tune_vm::Vm::new(host_resource_artifact())
        .with_host_executors(vec![executor])
        .with_host_resource_types(vec![resource_type]);

    assert!(matches!(
        vm.run_entry(),
        Err(tune_vm::VmFault {
            error: tune_vm::VmError::MissingHostAuthority { .. },
            ..
        })
    ));
}

#[test]
fn vm_runs_host_resource_cleanup_once() -> Result<(), &'static str> {
    let cleaned = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let cleaned_for_callback = std::sync::Arc::clone(&cleaned);
    let cleanup =
        tune_host::ResourceCleanupExecutor::new(move |resource: &tune_runtime::ResourceHandle| {
            cleaned_for_callback
                .lock()
                .map_err(|_| tune_host::HostCallError::new("cleanup log poisoned"))?
                .push(resource.id);
            Ok(())
        });
    let executor = tune_host::HostExecutor::new(|_: &[tune_runtime::Value]| {
        Ok(tune_runtime::Value::Tuple(vec![
            tune_runtime::Value::Resource(tune_runtime::ResourceHandle::new(
                tune_runtime::ResourceId(7),
                "fs.File",
            )),
            tune_runtime::Value::Resource(tune_runtime::ResourceHandle::new(
                tune_runtime::ResourceId(7),
                "fs.File",
            )),
        ]))
    });
    let resource_type =
        tune_vm::VmHostResourceType::new(tune_runtime::ResourceTypeId(0), "fs.File")
            .cleanup(tune_host::ResourceCleanup::HostCallback)
            .with_cleanup_executor(cleanup);
    let mut vm = tune_vm::Vm::new(host_resource_artifact())
        .with_host_executors(vec![executor])
        .with_host_resource_types(vec![resource_type]);

    let _value = vm.run_entry().map_err(|_| "vm should run entry")?;
    vm.cleanup_resources()
        .map_err(|_| "resource cleanup should succeed")?;
    vm.cleanup_resources()
        .map_err(|_| "resource cleanup should be idempotent")?;

    assert_eq!(
        cleaned
            .lock()
            .map_err(|_| "cleanup log should not be poisoned")?
            .as_slice(),
        &[tune_runtime::ResourceId(7)]
    );
    Ok(())
}
