use tune_host::{HostCallError, HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

const MAX_BYTES: u64 = 1_048_576;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "random",
        vec![
            HostFunction::new(
                "size",
                vec![
                    HostParam::new("seed", Shape::Size),
                    HostParam::new("index", Shape::Size),
                ],
                Shape::Size,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let seed = size_arg(args, 0, "seed")?;
                let index = size_arg(args, 1, "index")?;
                Ok(Value::Size(splitmix64(seed.wrapping_add(index))))
            }),
            HostFunction::new(
                "float",
                vec![
                    HostParam::new("seed", Shape::Size),
                    HostParam::new("index", Shape::Size),
                ],
                Shape::Float,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let seed = size_arg(args, 0, "seed")?;
                let index = size_arg(args, 1, "index")?;
                let value = splitmix64(seed.wrapping_add(index));
                Ok(Value::Float(unit_float(value)))
            }),
            HostFunction::new(
                "bool",
                vec![
                    HostParam::new("seed", Shape::Size),
                    HostParam::new("index", Shape::Size),
                ],
                Shape::Bool,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let seed = size_arg(args, 0, "seed")?;
                let index = size_arg(args, 1, "index")?;
                Ok(Value::Bool(splitmix64(seed.wrapping_add(index)) & 1 == 1))
            }),
            HostFunction::new(
                "index",
                vec![
                    HostParam::new("seed", Shape::Size),
                    HostParam::new("index", Shape::Size),
                    HostParam::new("len", Shape::Size),
                ],
                size_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let seed = size_arg(args, 0, "seed")?;
                let index = size_arg(args, 1, "index")?;
                let len = size_arg(args, 2, "len")?;
                if len == 0 {
                    return Ok(crate::result_error("random.index len must be > 0"));
                }
                Ok(crate::result_ok(Value::Size(
                    splitmix64(seed.wrapping_add(index)) % len,
                )))
            }),
            HostFunction::new(
                "int",
                vec![
                    HostParam::new("seed", Shape::Size),
                    HostParam::new("index", Shape::Size),
                    HostParam::new("min", Shape::Int),
                    HostParam::new("max", Shape::Int),
                ],
                int_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let seed = size_arg(args, 0, "seed")?;
                let index = size_arg(args, 1, "index")?;
                let min = int_arg(args, 2, "min")?;
                let max = int_arg(args, 3, "max")?;
                if min > max {
                    return Ok(crate::result_error("random.int min must be <= max"));
                }
                let span = (i128::from(max) - i128::from(min) + 1) as u128;
                let offset = u128::from(splitmix64(seed.wrapping_add(index))) % span;
                Ok(crate::result_ok(Value::Int(
                    (i128::from(min) + offset as i128) as i64,
                )))
            }),
            HostFunction::new(
                "bytes",
                vec![
                    HostParam::new("seed", Shape::Size),
                    HostParam::new("count", Shape::Size),
                ],
                bytes_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let seed = size_arg(args, 0, "seed")?;
                let count = size_arg(args, 1, "count")?;
                if count > MAX_BYTES {
                    return Ok(crate::result_error(format!(
                        "random.bytes count must be <= {MAX_BYTES}"
                    )));
                }
                let mut bytes = Vec::with_capacity(count as usize);
                let mut index = 0;
                while bytes.len() < count as usize {
                    for byte in splitmix64(seed.wrapping_add(index)).to_le_bytes() {
                        if bytes.len() == count as usize {
                            break;
                        }
                        bytes.push(Value::Byte(byte));
                    }
                    index += 1;
                }
                Ok(crate::result_ok(Value::Sequence(bytes)))
            }),
        ],
    )
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn unit_float(value: u64) -> f64 {
    const SCALE: f64 = (1_u64 << 53) as f64;
    ((value >> 11) as f64) / SCALE
}

fn int_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Int),
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

fn size_arg(args: &[Value], index: usize, name: &str) -> Result<u64, HostCallError> {
    match args.get(index) {
        Some(Value::Size(value)) => Ok(*value),
        None => Err(HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(HostCallError::new(format!("expected Size for `{name}`"))),
    }
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
