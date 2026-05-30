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
                "int_radix",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("radix", Shape::Size),
                ],
                result_shape(Shape::Int),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let radix = match radix_arg(args, 1)? {
                    Ok(radix) => radix,
                    Err(error) => return Ok(crate::result_error(error)),
                };
                Ok(match i64::from_str_radix(text, radix) {
                    Ok(value) => crate::result_ok(Value::Int(value)),
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
                "size_radix",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("radix", Shape::Size),
                ],
                result_shape(Shape::Size),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let radix = match radix_arg(args, 1)? {
                    Ok(radix) => radix,
                    Err(error) => return Ok(crate::result_error(error)),
                };
                Ok(match u64::from_str_radix(text, radix) {
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
            HostFunction::new(
                "byte_radix",
                vec![
                    HostParam::new("text", Shape::String),
                    HostParam::new("radix", Shape::Size),
                ],
                result_shape(Shape::Byte),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let radix = match radix_arg(args, 1)? {
                    Ok(radix) => radix,
                    Err(error) => return Ok(crate::result_error(error)),
                };
                Ok(match u8::from_str_radix(text, radix) {
                    Ok(value) => crate::result_ok(Value::Byte(value)),
                    Err(error) => crate::result_error(error.to_string()),
                })
            }),
            HostFunction::new(
                "bool",
                vec![HostParam::new("text", Shape::String)],
                result_shape(Shape::Bool),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(match text.parse::<bool>() {
                    Ok(value) => crate::result_ok(Value::Bool(value)),
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

fn radix_arg(
    args: &[Value],
    index: usize,
) -> Result<Result<u32, String>, tune_host::HostCallError> {
    let radix = match args.get(index) {
        Some(Value::Size(value)) => *value,
        None => {
            return Err(tune_host::HostCallError::new(format!(
                "missing argument `radix` at index {index}"
            )));
        }
        _ => return Err(tune_host::HostCallError::new("expected Size for `radix`")),
    };
    if !(2..=36).contains(&radix) {
        return Ok(Err("radix must be between 2 and 36".into()));
    }
    Ok(Ok(radix as u32))
}
