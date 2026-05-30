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
                    Ok(output) => Ok(crate::result_ok(Value::HostStruct {
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
                    })),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
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
