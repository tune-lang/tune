use std::process::Command;

use tune_host::{HostFunction, HostModule, HostParam, HostValueField, HostValueType};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "process",
        vec![
            HostFunction::new(
                "run",
                vec![
                    HostParam::new("command", Shape::String),
                    HostParam::new("args", Shape::Sequence(Box::new(Shape::String))),
                ],
                process_result_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("process.run".into())])
            .with_executor(|args: &[Value]| {
                let command = crate::string_arg(args, 0, "command")?;
                let command_args = crate::string_sequence_arg(args, 1, "args")?;
                match Command::new(command).args(command_args).output() {
                    Ok(output) => Ok(crate::result_ok(process_result_value(output))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "shell",
                vec![HostParam::new("command", Shape::String)],
                process_result_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("process.run".into())])
            .with_executor(|args: &[Value]| {
                let command = crate::string_arg(args, 0, "command")?;
                let mut shell = shell_command(command);
                match shell.output() {
                    Ok(output) => Ok(crate::result_ok(process_result_value(output))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "success",
                vec![HostParam::new("result", process_result_shape())],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let result = process_result_arg(args, 0, "result")?;
                let code = process_result_code(result)?;
                Ok(Value::Bool(code == 0))
            }),
            HostFunction::new(
                "code",
                vec![HostParam::new("result", process_result_shape())],
                Shape::Int,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let result = process_result_arg(args, 0, "result")?;
                Ok(Value::Int(process_result_code(result)?))
            }),
            HostFunction::new(
                "stdout",
                vec![HostParam::new("result", process_result_shape())],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let result = process_result_arg(args, 0, "result")?;
                Ok(Value::String(process_result_text(result, "stdout")?.into()))
            }),
            HostFunction::new(
                "stderr",
                vec![HostParam::new("result", process_result_shape())],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let result = process_result_arg(args, 0, "result")?;
                Ok(Value::String(process_result_text(result, "stderr")?.into()))
            }),
            HostFunction::new(
                "stdout_lines",
                vec![HostParam::new("result", process_result_shape())],
                Shape::Sequence(Box::new(Shape::String)),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let result = process_result_arg(args, 0, "result")?;
                let stdout = process_result_text(result, "stdout")?;
                Ok(Value::Sequence(
                    stdout
                        .lines()
                        .map(|line| Value::String(line.to_owned()))
                        .collect::<Vec<_>>(),
                ))
            }),
            HostFunction::new(
                "stderr_lines",
                vec![HostParam::new("result", process_result_shape())],
                Shape::Sequence(Box::new(Shape::String)),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let result = process_result_arg(args, 0, "result")?;
                let stderr = process_result_text(result, "stderr")?;
                Ok(Value::Sequence(
                    stderr
                        .lines()
                        .map(|line| Value::String(line.to_owned()))
                        .collect::<Vec<_>>(),
                ))
            }),
        ],
    )
    .with_values(vec![HostValueType::new(
        "ProcessResult",
        vec![
            HostValueField::new("code", Shape::Int),
            HostValueField::new("stdout", Shape::String),
            HostValueField::new("stderr", Shape::String),
        ],
    )])
}

fn process_result_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(process_result_shape()),
        err: Box::new(Shape::String),
    }
}

fn process_result_shape() -> Shape {
    Shape::Struct("process.ProcessResult".into())
}

fn process_result_value(output: std::process::Output) -> Value {
    Value::HostStruct {
        type_name: "process.ProcessResult".into(),
        fields: vec![
            (
                "code".into(),
                Value::Int(i64::from(output.status.code().unwrap_or(-1))),
            ),
            (
                "stdout".into(),
                Value::String(String::from_utf8_lossy(&output.stdout).to_string()),
            ),
            (
                "stderr".into(),
                Value::String(String::from_utf8_lossy(&output.stderr).to_string()),
            ),
        ],
    }
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("cmd");
    shell.arg("/C").arg(command);
    shell
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut shell = Command::new("sh");
    shell.arg("-c").arg(command);
    shell
}

fn process_result_arg<'a>(
    args: &'a [Value],
    index: usize,
    name: &str,
) -> Result<&'a [(String, Value)], tune_host::HostCallError> {
    match args.get(index) {
        Some(Value::HostStruct { type_name, fields }) if type_name == "process.ProcessResult" => {
            Ok(fields)
        }
        None => Err(tune_host::HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(tune_host::HostCallError::new(format!(
            "expected process.ProcessResult for `{name}`"
        ))),
    }
}

fn process_result_code(fields: &[(String, Value)]) -> Result<i64, tune_host::HostCallError> {
    match process_result_field(fields, "code")? {
        Value::Int(value) => Ok(*value),
        _ => Err(tune_host::HostCallError::new(
            "process.ProcessResult code field is not Int",
        )),
    }
}

fn process_result_text<'a>(
    fields: &'a [(String, Value)],
    name: &str,
) -> Result<&'a str, tune_host::HostCallError> {
    match process_result_field(fields, name)? {
        Value::String(value) => Ok(value),
        _ => Err(tune_host::HostCallError::new(format!(
            "process.ProcessResult {name} field is not String"
        ))),
    }
}

fn process_result_field<'a>(
    fields: &'a [(String, Value)],
    name: &str,
) -> Result<&'a Value, tune_host::HostCallError> {
    fields
        .iter()
        .find(|(field_name, _)| field_name == name)
        .map(|(_, value)| value)
        .ok_or_else(|| {
            tune_host::HostCallError::new(format!("process.ProcessResult missing `{name}`"))
        })
}
