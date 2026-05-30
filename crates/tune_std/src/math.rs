use tune_host::{HostCallError, HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "math",
        vec![
            unary_float("abs", f64::abs),
            binary_float("min", f64::min),
            binary_float("max", f64::max),
            HostFunction::new(
                "clamp",
                vec![
                    HostParam::new("value", Shape::Float),
                    HostParam::new("min", Shape::Float),
                    HostParam::new("max", Shape::Float),
                ],
                Shape::Float,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let value = float_arg(args, 0, "value")?;
                let min = float_arg(args, 1, "min")?;
                let max = float_arg(args, 2, "max")?;
                if min > max {
                    return Err(HostCallError::new("math.clamp min must be <= max"));
                }
                Ok(Value::Float(value.clamp(min, max)))
            }),
            unary_float("floor", f64::floor),
            unary_float("ceil", f64::ceil),
            unary_float("round", f64::round),
            unary_float("sqrt", f64::sqrt),
            binary_float("pow", f64::powf),
        ],
    )
}

fn unary_float(name: &'static str, op: fn(f64) -> f64) -> HostFunction {
    HostFunction::new(
        name,
        vec![HostParam::new("value", Shape::Float)],
        Shape::Float,
    )
    .task_safe(true)
    .with_executor(move |args: &[Value]| {
        let value = float_arg(args, 0, "value")?;
        Ok(Value::Float(op(value)))
    })
}

fn binary_float(name: &'static str, op: fn(f64, f64) -> f64) -> HostFunction {
    HostFunction::new(
        name,
        vec![
            HostParam::new("left", Shape::Float),
            HostParam::new("right", Shape::Float),
        ],
        Shape::Float,
    )
    .task_safe(true)
    .with_executor(move |args: &[Value]| {
        let left = float_arg(args, 0, "left")?;
        let right = float_arg(args, 1, "right")?;
        Ok(Value::Float(op(left, right)))
    })
}

fn float_arg(args: &[Value], index: usize, name: &str) -> Result<f64, HostCallError> {
    match args.get(index) {
        Some(Value::Float(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Float for `{name}`"))),
    }
}
