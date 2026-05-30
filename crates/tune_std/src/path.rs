use std::path::{Component, Path, PathBuf};

use tune_host::{HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "path",
        vec![
            HostFunction::new(
                "join",
                vec![
                    HostParam::new("base", Shape::String),
                    HostParam::new("next", Shape::String),
                ],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let (base, next) = crate::string_pair(args, "base", "next")?;
                Ok(Value::String(
                    Path::new(base).join(next).display().to_string(),
                ))
            }),
            HostFunction::new(
                "join_all",
                vec![HostParam::new(
                    "parts",
                    Shape::Sequence(Box::new(Shape::String)),
                )],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let parts = crate::string_sequence_arg(args, 0, "parts")?;
                let mut path = PathBuf::new();
                for part in parts {
                    path.push(part);
                }
                Ok(Value::String(path.display().to_string()))
            }),
            HostFunction::new(
                "ext",
                vec![HostParam::new("path", Shape::String)],
                optional_string_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(optional_path_part(Path::new(path).extension()))
            }),
            HostFunction::new(
                "stem",
                vec![HostParam::new("path", Shape::String)],
                optional_string_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(optional_path_part(Path::new(path).file_stem()))
            }),
            HostFunction::new(
                "file_name",
                vec![HostParam::new("path", Shape::String)],
                optional_string_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(optional_path_part(Path::new(path).file_name()))
            }),
            HostFunction::new(
                "parent",
                vec![HostParam::new("path", Shape::String)],
                optional_string_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(Path::new(path).parent().map_or(Value::None, |parent| {
                    Value::String(parent.display().to_string())
                }))
            }),
            HostFunction::new(
                "normalize",
                vec![HostParam::new("path", Shape::String)],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(Value::String(normalize_lexical(path).display().to_string()))
            }),
            HostFunction::new(
                "components",
                vec![HostParam::new("path", Shape::String)],
                Shape::Sequence(Box::new(Shape::String)),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(Value::Sequence(
                    Path::new(path)
                        .components()
                        .filter_map(|component| {
                            component
                                .as_os_str()
                                .to_str()
                                .map(|part| Value::String(part.to_owned()))
                        })
                        .collect::<Vec<_>>(),
                ))
            }),
            HostFunction::new(
                "with_ext",
                vec![
                    HostParam::new("path", Shape::String),
                    HostParam::new("ext", Shape::String),
                ],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                let ext = crate::string_arg(args, 1, "ext")?;
                Ok(Value::String(
                    Path::new(path).with_extension(ext).display().to_string(),
                ))
            }),
            HostFunction::new(
                "is_absolute",
                vec![HostParam::new("path", Shape::String)],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(Value::Bool(Path::new(path).is_absolute()))
            }),
            HostFunction::new(
                "is_relative",
                vec![HostParam::new("path", Shape::String)],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                Ok(Value::Bool(Path::new(path).is_relative()))
            }),
            HostFunction::new("separator", Vec::new(), Shape::String)
                .task_safe(true)
                .with_executor(|_: &[Value]| Ok(Value::String(std::path::MAIN_SEPARATOR.into()))),
        ],
    )
}

fn optional_string_shape() -> Shape {
    Shape::Optional(Box::new(Shape::String))
}

fn optional_path_part(part: Option<&std::ffi::OsStr>) -> Value {
    part.and_then(std::ffi::OsStr::to_str)
        .map_or(Value::None, |part| Value::String(part.to_owned()))
}

fn normalize_lexical(path: &str) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match normalized.components().next_back() {
                Some(Component::Normal(_)) => {
                    normalized.pop();
                }
                Some(Component::ParentDir) | None => normalized.push(".."),
                Some(Component::Prefix(_) | Component::RootDir | Component::CurDir) => {}
            },
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => normalized.push(part),
        }
    }

    if normalized.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        normalized
    }
}
