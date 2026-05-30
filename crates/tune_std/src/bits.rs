use tune_host::{HostCallError, HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "bits",
        vec![
            size_from_int("count_ones", i64::count_ones),
            size_from_int("leading_zeros", i64::leading_zeros),
            size_from_int("trailing_zeros", i64::trailing_zeros),
            rotate("rotate_left", i64::rotate_left),
            rotate("rotate_right", i64::rotate_right),
        ],
    )
}

fn size_from_int(name: &'static str, op: fn(i64) -> u32) -> HostFunction {
    HostFunction::new(name, vec![HostParam::new("value", Shape::Int)], Shape::Size)
        .task_safe(true)
        .with_executor(move |args: &[Value]| {
            let value = int_arg(args, 0, "value")?;
            Ok(Value::Size(u64::from(op(value))))
        })
}

fn rotate(name: &'static str, op: fn(i64, u32) -> i64) -> HostFunction {
    HostFunction::new(
        name,
        vec![
            HostParam::new("value", Shape::Int),
            HostParam::new("amount", Shape::Size),
        ],
        Shape::Int,
    )
    .task_safe(true)
    .with_executor(move |args: &[Value]| {
        let value = int_arg(args, 0, "value")?;
        let amount = size_arg(args, 1, "amount")?;
        Ok(Value::Int(op(value, (amount % 64) as u32)))
    })
}

fn int_arg(args: &[Value], index: usize, name: &str) -> Result<i64, HostCallError> {
    match args.get(index) {
        Some(Value::Int(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Int for `{name}`"))),
    }
}

fn size_arg(args: &[Value], index: usize, name: &str) -> Result<u64, HostCallError> {
    match args.get(index) {
        Some(Value::Size(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Size for `{name}`"))),
    }
}
