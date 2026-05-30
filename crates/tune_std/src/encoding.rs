use tune_host::{HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "encoding",
        vec![
            HostFunction::new(
                "hex",
                vec![HostParam::new(
                    "data",
                    Shape::Sequence(Box::new(Shape::Byte)),
                )],
                Shape::String,
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let bytes = crate::byte_sequence_arg(args, 0, "data")?;
                Ok(Value::String(hex_encode(&bytes)))
            }),
            HostFunction::new(
                "from_hex",
                vec![HostParam::new("text", Shape::String)],
                bytes_result_shape(),
            )
            .task_safe(true)
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                match hex_decode(text) {
                    Ok(bytes) => Ok(crate::result_ok(Value::Sequence(
                        bytes.into_iter().map(Value::Byte).collect::<Vec<_>>(),
                    ))),
                    Err(error) => Ok(crate::result_error(error)),
                }
            }),
        ],
    )
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    out
}

fn hex_decode(text: &str) -> Result<Vec<u8>, String> {
    if !text.len().is_multiple_of(2) {
        return Err("hex input must contain an even number of digits".into());
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let mut chars = text.bytes();
    while let Some(high) = chars.next() {
        let low = chars
            .next()
            .ok_or_else(|| "hex input must contain an even number of digits".to_owned())?;
        bytes.push((hex_value(high)? << 4) | hex_value(low)?);
    }
    Ok(bytes)
}

fn hex_value(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(format!("invalid hex digit `{}`", char::from(byte))),
    }
}

fn bytes_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Sequence(Box::new(Shape::Byte))),
        err: Box::new(Shape::String),
    }
}
