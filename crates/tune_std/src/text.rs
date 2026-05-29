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
