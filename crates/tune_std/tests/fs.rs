use tune_host::HostContext;

fn fs_executor<'a>(
    module: &'a tune_host::HostModule,
    name: &str,
) -> Result<&'a tune_host::HostExecutor, &'static str> {
    module
        .functions
        .iter()
        .find(|function| function.name == name)
        .and_then(|function| function.executor.as_ref())
        .ok_or("fs function should carry an executor")
}

#[test]
fn fs_byte_executors_return_result_values() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let path = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}.bin",
        std::process::id(),
        "fs-bytes"
    ));
    let path_text = path.to_string_lossy().to_string();

    let write_result = fs_executor(&module, "write_bytes")?
        .call(&[
            tune_runtime::Value::String(path_text.clone()),
            tune_runtime::Value::Sequence(vec![
                tune_runtime::Value::Byte(1),
                tune_runtime::Value::Byte(2),
                tune_runtime::Value::Byte(3),
            ]),
        ])
        .map_err(|_| "fs.write_bytes should execute")?;
    assert!(matches!(
        write_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));

    let exists = fs_executor(&module, "exists")?
        .call(&[tune_runtime::Value::String(path_text.clone())])
        .map_err(|_| "fs.exists should execute")?;
    assert_eq!(exists, tune_runtime::Value::Bool(true));

    let is_file = fs_executor(&module, "is_file")?
        .call(&[tune_runtime::Value::String(path_text.clone())])
        .map_err(|_| "fs.is_file should execute")?;
    assert_eq!(is_file, tune_runtime::Value::Bool(true));

    let is_dir = fs_executor(&module, "is_dir")?
        .call(&[tune_runtime::Value::String(path_text.clone())])
        .map_err(|_| "fs.is_dir should execute")?;
    assert_eq!(is_dir, tune_runtime::Value::Bool(false));

    let read_result = fs_executor(&module, "read_bytes")?
        .call(&[tune_runtime::Value::String(path_text)])
        .map_err(|_| "fs.read_bytes should execute")?;
    assert_eq!(
        read_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Sequence(vec![
                tune_runtime::Value::Byte(1),
                tune_runtime::Value::Byte(2),
                tune_runtime::Value::Byte(3),
            ])],
            propagation_frames: Vec::new(),
        }
    );

    drop(std::fs::remove_file(path));
    Ok(())
}

#[test]
fn fs_mutation_executors_return_result_values() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let root = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}",
        std::process::id(),
        "fs-mutation"
    ));
    let file = root.join("data.txt");
    let root_text = root.to_string_lossy().to_string();
    let file_text = file.to_string_lossy().to_string();

    let create_result = fs_executor(&module, "create_dir")?
        .call(&[tune_runtime::Value::String(root_text.clone())])
        .map_err(|_| "fs.create_dir should execute")?;
    assert!(matches!(
        create_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));

    fs_executor(&module, "write_text")?
        .call(&[
            tune_runtime::Value::String(file_text.clone()),
            tune_runtime::Value::String("temporary".into()),
        ])
        .map_err(|_| "fs.write_text should execute")?;

    let remove_file_result = fs_executor(&module, "remove_file")?
        .call(&[tune_runtime::Value::String(file_text)])
        .map_err(|_| "fs.remove_file should execute")?;
    assert!(matches!(
        remove_file_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));

    let remove_dir_result = fs_executor(&module, "remove_dir")?
        .call(&[tune_runtime::Value::String(root_text)])
        .map_err(|_| "fs.remove_dir should execute")?;
    assert!(matches!(
        remove_dir_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));

    drop(std::fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn fs_recursive_directory_executors_return_result_values() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let root = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}",
        std::process::id(),
        "fs-recursive"
    ));
    let nested = root.join("a").join("b");
    let root_text = root.to_string_lossy().to_string();
    let nested_text = nested.to_string_lossy().to_string();

    let create_result = fs_executor(&module, "create_dir_all")?
        .call(&[tune_runtime::Value::String(nested_text.clone())])
        .map_err(|_| "fs.create_dir_all should execute")?;
    assert!(matches!(
        create_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));
    assert!(nested.is_dir());

    let is_dir = fs_executor(&module, "is_dir")?
        .call(&[tune_runtime::Value::String(nested_text)])
        .map_err(|_| "fs.is_dir should execute")?;
    assert_eq!(is_dir, tune_runtime::Value::Bool(true));

    let remove_result = fs_executor(&module, "remove_dir_all")?
        .call(&[tune_runtime::Value::String(root_text.clone())])
        .map_err(|_| "fs.remove_dir_all should execute")?;
    assert!(matches!(
        remove_result,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));
    assert!(!root.exists());

    drop(std::fs::remove_dir_all(root_text));
    Ok(())
}

#[test]
fn fs_read_dir_executor_returns_dir_entry_host_values() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let root = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}",
        std::process::id(),
        "fs-read-dir"
    ));
    std::fs::create_dir(&root).map_err(|_| "fixture dir should create")?;
    let child = root.join("child.txt");
    std::fs::write(&child, "child").map_err(|_| "fixture child should write")?;

    let read_dir = fs_executor(&module, "read_dir")?;
    let value = read_dir
        .call(&[tune_runtime::Value::String(
            root.to_string_lossy().to_string(),
        )])
        .map_err(|_| "fs.read_dir should execute")?;

    let tune_runtime::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultOk,
        fields,
        ..
    } = value
    else {
        return Err("fs.read_dir should return Ok");
    };
    let Some(tune_runtime::Value::Sequence(entries)) = fields.first() else {
        return Err("fs.read_dir Ok payload should be a sequence");
    };
    assert_eq!(entries.len(), 1);
    let tune_runtime::Value::HostStruct { type_name, fields } = &entries[0] else {
        return Err("fs.read_dir entries should be host structs");
    };
    assert_eq!(type_name, "fs.DirEntry");
    assert!(fields.iter().any(|(name, value)| {
        name == "name" && value == &tune_runtime::Value::String("child.txt".into())
    }));

    drop(std::fs::remove_dir_all(root));
    Ok(())
}

#[test]
fn fs_metadata_executor_returns_metadata_host_value() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let path = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}.txt",
        std::process::id(),
        "fs-metadata"
    ));
    std::fs::write(&path, "hello").map_err(|_| "fixture file should write")?;

    let metadata = fs_executor(&module, "metadata")?
        .call(&[tune_runtime::Value::String(
            path.to_string_lossy().to_string(),
        )])
        .map_err(|_| "fs.metadata should execute")?;
    let tune_runtime::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultOk,
        fields,
        ..
    } = metadata
    else {
        return Err("fs.metadata should return Ok");
    };
    let Some(tune_runtime::Value::HostStruct { type_name, fields }) = fields.first() else {
        return Err("fs.metadata Ok payload should be a host struct");
    };
    assert_eq!(type_name, "fs.Metadata");
    assert!(
        fields
            .iter()
            .any(|(name, value)| name == "len" && value == &tune_runtime::Value::Size(5))
    );
    assert!(
        fields
            .iter()
            .any(|(name, value)| name == "is_file" && value == &tune_runtime::Value::Bool(true))
    );

    drop(std::fs::remove_file(path));
    Ok(())
}

#[test]
fn fs_copy_rename_and_append_executors_return_result_values() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let root = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}",
        std::process::id(),
        "fs-file-ops"
    ));
    std::fs::create_dir(&root).map_err(|_| "fixture dir should create")?;
    let source = root.join("source.txt");
    let copied = root.join("copied.txt");
    let renamed = root.join("renamed.txt");
    std::fs::write(&source, "hello").map_err(|_| "fixture source should write")?;

    let append = fs_executor(&module, "append_text")?
        .call(&[
            tune_runtime::Value::String(source.to_string_lossy().to_string()),
            tune_runtime::Value::String(" std".into()),
        ])
        .map_err(|_| "fs.append_text should execute")?;
    assert!(matches!(
        append,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));
    assert_eq!(
        std::fs::read_to_string(&source).map_err(|_| "source should read")?,
        "hello std"
    );

    let copy = fs_executor(&module, "copy")?
        .call(&[
            tune_runtime::Value::String(source.to_string_lossy().to_string()),
            tune_runtime::Value::String(copied.to_string_lossy().to_string()),
        ])
        .map_err(|_| "fs.copy should execute")?;
    assert!(matches!(
        copy,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields,
            ..
        } if matches!(fields.as_slice(), [tune_runtime::Value::Size(9)])
    ));

    let rename = fs_executor(&module, "rename")?
        .call(&[
            tune_runtime::Value::String(copied.to_string_lossy().to_string()),
            tune_runtime::Value::String(renamed.to_string_lossy().to_string()),
        ])
        .map_err(|_| "fs.rename should execute")?;
    assert!(matches!(
        rename,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));
    assert!(renamed.exists());
    assert!(!copied.exists());

    drop(std::fs::remove_dir_all(root));
    Ok(())
}

#[derive(Default)]
struct TestHostContext {
    next: std::sync::atomic::AtomicU64,
    objects: std::sync::Mutex<Vec<(tune_runtime::ResourceHandle, tune_host::HostResourceObject)>>,
}

impl tune_host::HostContext for TestHostContext {
    fn insert_resource(
        &self,
        type_name: &str,
        object: tune_host::HostResourceObject,
    ) -> Result<tune_runtime::ResourceHandle, tune_host::HostCallError> {
        let id = self.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let handle = tune_runtime::ResourceHandle::new(tune_runtime::ResourceId(id), type_name);
        self.objects
            .lock()
            .map_err(|_| tune_host::HostCallError::new("test resource table is poisoned"))?
            .push((handle.clone(), object));
        Ok(handle)
    }

    fn get_resource(
        &self,
        handle: &tune_runtime::ResourceHandle,
    ) -> Result<tune_host::HostResourceObject, tune_host::HostCallError> {
        self.objects
            .lock()
            .map_err(|_| tune_host::HostCallError::new("test resource table is poisoned"))?
            .iter()
            .find(|(stored, _)| stored.id == handle.id)
            .map(|(_, object)| object.clone())
            .ok_or_else(|| tune_host::HostCallError::new("unknown test resource"))
    }

    fn close_resource(
        &self,
        handle: &tune_runtime::ResourceHandle,
    ) -> Result<(), tune_host::HostCallError> {
        let mut objects = self
            .objects
            .lock()
            .map_err(|_| tune_host::HostCallError::new("test resource table is poisoned"))?;
        let Some(index) = objects
            .iter()
            .position(|(stored, _)| stored.id == handle.id)
        else {
            return Err(tune_host::HostCallError::new("unknown test resource"));
        };
        objects.remove(index);
        Ok(())
    }
}

#[test]
fn fs_resource_executors_use_host_context() -> Result<(), &'static str> {
    let module = tune_std::fs::install();
    let context = TestHostContext::default();
    let path = std::env::temp_dir().join(format!(
        "dyno-tune-std-{}-{}.txt",
        std::process::id(),
        "fs-resource"
    ));
    std::fs::write(&path, "abcdef").map_err(|_| "fixture file should write")?;
    let path_text = path.to_string_lossy().to_string();

    let open = module
        .functions
        .iter()
        .find(|function| function.name == "open")
        .and_then(|function| function.executor.as_ref())
        .ok_or("fs.open should carry an executor")?;
    let opened = open
        .call_with_context(&[tune_runtime::Value::String(path_text)], &context)
        .map_err(|_| "fs.open should execute")?;
    let tune_runtime::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultOk,
        fields,
        ..
    } = opened
    else {
        return Err("fs.open should return Ok");
    };
    let Some(tune_runtime::Value::Resource(handle)) = fields.first() else {
        return Err("fs.open Ok payload should be a resource");
    };

    let read_chunk = module
        .functions
        .iter()
        .find(|function| function.name == "read_chunk")
        .and_then(|function| function.executor.as_ref())
        .ok_or("fs.read_chunk should carry an executor")?;
    let chunk = read_chunk
        .call_with_context(
            &[
                tune_runtime::Value::Resource(handle.clone()),
                tune_runtime::Value::Size(3),
            ],
            &context,
        )
        .map_err(|_| "fs.read_chunk should execute")?;
    assert_eq!(
        chunk,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            fields: vec![tune_runtime::Value::Sequence(vec![
                tune_runtime::Value::Byte(b'a'),
                tune_runtime::Value::Byte(b'b'),
                tune_runtime::Value::Byte(b'c'),
            ])],
            propagation_frames: Vec::new(),
        }
    );

    let close = module
        .functions
        .iter()
        .find(|function| function.name == "close")
        .and_then(|function| function.executor.as_ref())
        .ok_or("fs.close should carry an executor")?;
    let closed = close
        .call_with_context(&[tune_runtime::Value::Resource(handle.clone())], &context)
        .map_err(|_| "fs.close should execute")?;
    assert!(matches!(
        closed,
        tune_runtime::Value::Variant {
            variant: tune_runtime::value::RuntimeVariant::ResultOk,
            ..
        }
    ));
    assert!(context.get_resource(handle).is_err());

    drop(std::fs::remove_file(path));
    Ok(())
}
