use tune_host::{HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "text",
        vec![
            HostFunction::new(
                "byte_len",
                vec![HostParam::new("text", Shape::String)],
                Shape::Size,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::Size(text.len() as u64))
            }),
            HostFunction::new(
                "bytes",
                vec![HostParam::new("text", Shape::String)],
                Shape::Sequence(Box::new(Shape::Byte)),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::Sequence(
                    text.bytes().map(Value::Byte).collect::<Vec<_>>(),
                ))
            }),
            HostFunction::new(
                "from_utf8",
                vec![HostParam::new(
                    "data",
                    Shape::Sequence(Box::new(Shape::Byte)),
                )],
                string_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let bytes = crate::byte_sequence_arg(args, 0, "data")?;
                match String::from_utf8(bytes) {
                    Ok(text) => Ok(crate::result_ok(Value::String(text))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "contains",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("needle", Shape::String),
                ],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let (text, needle) = crate::string_pair(args, "text", "needle")?;
                Ok(Value::Bool(text.contains(needle)))
            }),
            HostFunction::new(
                "trim",
                vec![HostParam::new("text", Shape::String)],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::String(text.trim().to_owned()))
            }),
            HostFunction::new(
                "lower",
                vec![HostParam::new("text", Shape::String)],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::String(text.to_lowercase()))
            }),
            HostFunction::new(
                "upper",
                vec![HostParam::new("text", Shape::String)],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::String(text.to_uppercase()))
            }),
            HostFunction::new(
                "replace",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("from", Shape::String),
                    HostParam::new("to", Shape::String),
                ],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let from = crate::string_arg(args, 1, "from")?;
                let to = crate::string_arg(args, 2, "to")?;
                Ok(Value::String(text.replace(from, to)))
            }),
            HostFunction::new(
                "split",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("delimiter", Shape::String),
                ],
                string_sequence_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let delimiter = crate::string_arg(args, 1, "delimiter")?;
                Ok(Value::Sequence(
                    text.split(delimiter)
                        .map(|part| Value::String(part.to_owned()))
                        .collect::<Vec<_>>(),
                ))
            }),
            HostFunction::new(
                "lines",
                vec![HostParam::new("text", Shape::String)],
                string_sequence_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::Sequence(
                    text.lines()
                        .map(|line| Value::String(line.to_owned()))
                        .collect::<Vec<_>>(),
                ))
            }),
            HostFunction::new(
                "starts_with",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("prefix", Shape::String),
                ],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let (text, prefix) = crate::string_pair(args, "text", "prefix")?;
                Ok(Value::Bool(text.starts_with(prefix)))
            }),
            HostFunction::new(
                "ends_with",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("suffix", Shape::String),
                ],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let (text, suffix) = crate::string_pair(args, "text", "suffix")?;
                Ok(Value::Bool(text.ends_with(suffix)))
            }),
        ],
    )
}

fn string_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::String),
        err: Box::new(Shape::String),
    }
}

fn string_sequence_shape() -> Shape {
    Shape::Sequence(Box::new(Shape::String))
}
