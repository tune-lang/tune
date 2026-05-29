use tune_host::{HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "parse",
        vec![
            HostFunction::new(
                "int",
                vec![HostParam::new("text", Shape::String)],
                result_shape(Shape::Int),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(match text.parse::<i64>() {
                    Ok(value) => crate::result_ok(Value::Int(value)),
                    Err(error) => crate::result_error(error.to_string()),
                })
            }),
            HostFunction::new(
                "float",
                vec![HostParam::new("text", Shape::String)],
                result_shape(Shape::Float),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(match text.parse::<f64>() {
                    Ok(value) => crate::result_ok(Value::Float(value)),
                    Err(error) => crate::result_error(error.to_string()),
                })
            }),
            HostFunction::new(
                "size",
                vec![HostParam::new("text", Shape::String)],
                result_shape(Shape::Size),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(match text.parse::<u64>() {
                    Ok(value) => crate::result_ok(Value::Size(value)),
                    Err(error) => crate::result_error(error.to_string()),
                })
            }),
            HostFunction::new(
                "byte",
                vec![HostParam::new("text", Shape::String)],
                result_shape(Shape::Byte),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(match text.parse::<u8>() {
                    Ok(value) => crate::result_ok(Value::Byte(value)),
                    Err(error) => crate::result_error(error.to_string()),
                })
            }),
        ],
    )
}

fn result_shape(ok: Shape) -> Shape {
    Shape::Result {
        ok: Box::new(ok),
        err: Box::new(Shape::String),
    }
}
