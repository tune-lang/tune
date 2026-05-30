use std::io::Read;
use std::sync::Mutex;

use tune_host::{
    HostContext, HostFunction, HostModule, HostParam, HostResourceType, HostValueField,
    HostValueType,
};
use tune_runtime::ResourceHandle;
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "fs",
        vec![
            HostFunction::new(
                "read_text",
                vec![HostParam::new("path", Shape::String)],
                string_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::read_to_string(path) {
                    Ok(text) => Ok(crate::result_ok(Value::String(text))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "read_bytes",
                vec![HostParam::new("path", Shape::String)],
                bytes_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::read(path) {
                    Ok(bytes) => Ok(crate::result_ok(Value::Sequence(
                        bytes.into_iter().map(Value::Byte).collect::<Vec<_>>(),
                    ))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "exists",
                vec![HostParam::new("path", Shape::String)],
                Shape::Bool,
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(Value::Bool(std::path::Path::new(path).exists()))
            }),
            HostFunction::new(
                "read_dir",
                vec![HostParam::new("path", Shape::String)],
                dir_entries_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                let entries = match std::fs::read_dir(path) {
                    Ok(entries) => entries,
                    Err(error) => return Ok(crate::result_error(error.to_string())),
                };
                let mut values = Vec::new();
                for entry in entries {
                    let entry = match entry {
                        Ok(entry) => entry,
                        Err(error) => return Ok(crate::result_error(error.to_string())),
                    };
                    let is_dir = match entry.file_type() {
                        Ok(file_type) => file_type.is_dir(),
                        Err(error) => return Ok(crate::result_error(error.to_string())),
                    };
                    values.push(Value::HostStruct {
                        type_name: "fs.DirEntry".into(),
                        fields: vec![
                            (
                                "name".into(),
                                Value::String(entry.file_name().to_string_lossy().to_string()),
                            ),
                            (
                                "path".into(),
                                Value::String(entry.path().to_string_lossy().to_string()),
                            ),
                            ("is_dir".into(), Value::Bool(is_dir)),
                        ],
                    });
                }
                Ok(crate::result_ok(Value::Sequence(values)))
            }),
            HostFunction::new(
                "metadata",
                vec![HostParam::new("path", Shape::String)],
                metadata_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::metadata(path) {
                    Ok(metadata) => Ok(crate::result_ok(metadata_value(&metadata))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "open",
                vec![HostParam::new("path", Shape::String)],
                file_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_context_executor(|args: &[Value], context: &dyn HostContext| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::File::open(path) {
                    Ok(file) => {
                        let handle = context
                            .insert_resource("fs.File", std::sync::Arc::new(Mutex::new(file)))?;
                        Ok(crate::result_ok(Value::Resource(handle)))
                    }
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "read_chunk",
                vec![
                    HostParam::new("file", file_shape()),
                    HostParam::new("size", Shape::Size),
                ],
                bytes_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_context_executor(|args: &[Value], context: &dyn HostContext| {
                let handle = resource_arg(args, 0, "file")?;
                let size = size_arg(args, 1, "size")?;
                let object = context.get_resource(handle)?;
                let file = tune_host::downcast_resource::<Mutex<std::fs::File>>(object)?;
                let mut file = file
                    .lock()
                    .map_err(|_| tune_host::HostCallError::new("fs.File lock is poisoned"))?;
                let mut buffer = vec![0; size as usize];
                match file.read(&mut buffer) {
                    Ok(count) => {
                        buffer.truncate(count);
                        Ok(crate::result_ok(Value::Sequence(
                            buffer.into_iter().map(Value::Byte).collect::<Vec<_>>(),
                        )))
                    }
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "close",
                vec![HostParam::new("file", file_shape())],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_context_executor(|args: &[Value], context: &dyn HostContext| {
                let handle = resource_arg(args, 0, "file")?;
                match context.close_resource(handle) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.message)),
                }
            }),
            HostFunction::new(
                "write_text",
                vec![
                    HostParam::new("path", Shape::String),
                    HostParam::new("text", Shape::String),
                ],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                let text = crate::string_arg(args, 1, "text")?;
                match std::fs::write(path, text) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "append_text",
                vec![
                    HostParam::new("path", Shape::String),
                    HostParam::new("text", Shape::String),
                ],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                let text = crate::string_arg(args, 1, "text")?;
                match std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .and_then(|mut file| std::io::Write::write_all(&mut file, text.as_bytes()))
                {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "write_bytes",
                vec![
                    HostParam::new("path", Shape::String),
                    HostParam::new("data", Shape::Sequence(Box::new(Shape::Byte))),
                ],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                let bytes = crate::byte_sequence_arg(args, 1, "data")?;
                match std::fs::write(path, bytes) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "copy",
                vec![
                    HostParam::new("from", Shape::String),
                    HostParam::new("to", Shape::String),
                ],
                size_result_shape(),
            )
            .with_authorities(vec![
                tune_host::Authority("fs.read".into()),
                tune_host::Authority("fs.write".into()),
            ])
            .with_executor(|args: &[Value]| {
                let from = crate::string_arg(args, 0, "from")?;
                let to = crate::string_arg(args, 1, "to")?;
                match std::fs::copy(from, to) {
                    Ok(bytes) => Ok(crate::result_ok(Value::Size(bytes))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "rename",
                vec![
                    HostParam::new("from", Shape::String),
                    HostParam::new("to", Shape::String),
                ],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let from = crate::string_arg(args, 0, "from")?;
                let to = crate::string_arg(args, 1, "to")?;
                match std::fs::rename(from, to) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "create_dir",
                vec![HostParam::new("path", Shape::String)],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::create_dir(path) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "remove_file",
                vec![HostParam::new("path", Shape::String)],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::remove_file(path) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "remove_dir",
                vec![HostParam::new("path", Shape::String)],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::remove_dir(path) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
        ],
    )
    .with_values(vec![
        HostValueType::new(
            "DirEntry",
            vec![
                HostValueField::new("name", Shape::String),
                HostValueField::new("path", Shape::String),
                HostValueField::new("is_dir", Shape::Bool),
            ],
        ),
        HostValueType::new(
            "Metadata",
            vec![
                HostValueField::new("len", Shape::Size),
                HostValueField::new("is_file", Shape::Bool),
                HostValueField::new("is_dir", Shape::Bool),
                HostValueField::new("readonly", Shape::Bool),
            ],
        ),
    ])
    .with_resources(vec![
        HostResourceType::new("File", Shape::Struct("fs.File".into()))
            .with_authorities(vec![
                tune_host::Authority("fs.read".into()),
                tune_host::Authority("fs.write".into()),
            ])
            .retention(tune_host::ResourceRetention::HostRetained)
            .cleanup(tune_host::ResourceCleanup::HostCallback),
    ])
}

fn string_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::String),
        err: Box::new(Shape::String),
    }
}

fn unit_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Unit),
        err: Box::new(Shape::String),
    }
}

fn size_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Size),
        err: Box::new(Shape::String),
    }
}

fn bytes_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Sequence(Box::new(Shape::Byte))),
        err: Box::new(Shape::String),
    }
}

fn dir_entries_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Sequence(Box::new(dir_entry_shape()))),
        err: Box::new(Shape::String),
    }
}

fn dir_entry_shape() -> Shape {
    Shape::Struct("fs.DirEntry".into())
}

fn metadata_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(metadata_shape()),
        err: Box::new(Shape::String),
    }
}

fn metadata_shape() -> Shape {
    Shape::Struct("fs.Metadata".into())
}

fn metadata_value(metadata: &std::fs::Metadata) -> Value {
    Value::HostStruct {
        type_name: "fs.Metadata".into(),
        fields: vec![
            ("len".into(), Value::Size(metadata.len())),
            ("is_file".into(), Value::Bool(metadata.is_file())),
            ("is_dir".into(), Value::Bool(metadata.is_dir())),
            (
                "readonly".into(),
                Value::Bool(metadata.permissions().readonly()),
            ),
        ],
    }
}

fn file_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(file_shape()),
        err: Box::new(Shape::String),
    }
}

fn file_shape() -> Shape {
    Shape::Struct("fs.File".into())
}

fn resource_arg<'a>(
    args: &'a [Value],
    index: usize,
    name: &str,
) -> Result<&'a ResourceHandle, tune_host::HostCallError> {
    match args.get(index) {
        Some(Value::Resource(handle)) => Ok(handle),
        None => Err(tune_host::HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(tune_host::HostCallError::new(format!(
            "expected resource for `{name}`"
        ))),
    }
}

fn size_arg(args: &[Value], index: usize, name: &str) -> Result<u64, tune_host::HostCallError> {
    match args.get(index) {
        Some(Value::Size(value)) => Ok(*value),
        None => Err(tune_host::HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(tune_host::HostCallError::new(format!(
            "expected Size for `{name}`"
        ))),
    }
}
