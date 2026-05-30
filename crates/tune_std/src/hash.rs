use tune_host::{HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "hash",
        vec![
            HostFunction::new(
                "text",
                vec![HostParam::new("text", Shape::String)],
                Shape::Size,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                Ok(Value::Size(fnv1a64(text.as_bytes())))
            }),
            HostFunction::new(
                "bytes",
                vec![HostParam::new(
                    "data",
                    Shape::Sequence(Box::new(Shape::Byte)),
                )],
                Shape::Size,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let bytes = crate::byte_sequence_arg(args, 0, "data")?;
                Ok(Value::Size(fnv1a64(&bytes)))
            }),
            HostFunction::new(
                "combine",
                vec![
                    HostParam::new("left", Shape::Size),
                    HostParam::new("right", Shape::Size),
                ],
                Shape::Size,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let left = size_arg(args, 0, "left")?;
                let right = size_arg(args, 1, "right")?;
                Ok(Value::Size(fnv1a64_pair(left, right)))
            }),
        ],
    )
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn fnv1a64_pair(left: u64, right: u64) -> u64 {
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&left.to_le_bytes());
    bytes[8..].copy_from_slice(&right.to_le_bytes());
    fnv1a64(&bytes)
}

fn size_arg(args: &[Value], index: usize, name: &str) -> Result<u64, tune_host::HostCallError> {
    match args.get(index) {
        Some(Value::Size(value)) => Ok(*value),
        None => Err(tune_host::HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(tune_host::HostCallError::new(format!(
            "expected Size for `{name}`"
        ))),
    }
}
