pub mod bits;
pub mod env;
pub mod fs;
pub mod io;
pub mod json;
pub mod math;
pub mod meta;
pub mod parse;
pub mod path;
pub mod prelude;
pub mod process;
pub mod text;

#[derive(Debug, Clone, Copy, Default)]
pub struct StdHost;

impl tune_host::Host for StdHost {
    fn modules(&self) -> Vec<tune_host::HostModule> {
        modules()
    }
}

#[must_use]
pub fn host() -> StdHost {
    StdHost
}

#[must_use]
pub fn modules() -> Vec<tune_host::HostModule> {
    vec![
        io::install(),
        math::install(),
        bits::install(),
        parse::install(),
        text::install(),
        path::install(),
        env::install(),
        fs::install(),
        process::install(),
    ]
}

#[must_use]
pub(crate) fn result_ok(value: tune_runtime::Value) -> tune_runtime::Value {
    tune_runtime::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultOk,
        fields: vec![value],
        propagation_frames: Vec::new(),
    }
}

#[must_use]
pub(crate) fn result_error(message: impl Into<String>) -> tune_runtime::Value {
    tune_runtime::Value::Variant {
        variant: tune_runtime::value::RuntimeVariant::ResultError,
        fields: vec![tune_runtime::Value::String(message.into())],
        propagation_frames: Vec::new(),
    }
}

pub(crate) fn string_arg<'a>(
    args: &'a [tune_runtime::Value],
    index: usize,
    name: &str,
) -> Result<&'a str, tune_host::HostCallError> {
    match args.get(index) {
        Some(tune_runtime::Value::String(value)) => Ok(value),
        None => Err(tune_host::HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(tune_host::HostCallError::new(format!(
            "expected String for `{name}`"
        ))),
    }
}

pub(crate) fn string_pair<'a>(
    args: &'a [tune_runtime::Value],
    left: &str,
    right: &str,
) -> Result<(&'a str, &'a str), tune_host::HostCallError> {
    Ok((string_arg(args, 0, left)?, string_arg(args, 1, right)?))
}

pub(crate) fn byte_sequence_arg(
    args: &[tune_runtime::Value],
    index: usize,
    name: &str,
) -> Result<Vec<u8>, tune_host::HostCallError> {
    match args.get(index) {
        Some(tune_runtime::Value::Sequence(values)) => values
            .iter()
            .map(|value| match value {
                tune_runtime::Value::Byte(byte) => Ok(*byte),
                _ => Err(tune_host::HostCallError::new(format!(
                    "expected Byte items for `{name}`"
                ))),
            })
            .collect(),
        None => Err(tune_host::HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(tune_host::HostCallError::new(format!(
            "expected [Byte] for `{name}`"
        ))),
    }
}

pub(crate) fn string_sequence_arg(
    args: &[tune_runtime::Value],
    index: usize,
    name: &str,
) -> Result<Vec<String>, tune_host::HostCallError> {
    match args.get(index) {
        Some(tune_runtime::Value::Sequence(values)) => values
            .iter()
            .map(|value| match value {
                tune_runtime::Value::String(text) => Ok(text.clone()),
                _ => Err(tune_host::HostCallError::new(format!(
                    "expected String items for `{name}`"
                ))),
            })
            .collect(),
        None => Err(tune_host::HostCallError::new(format!(
            "missing argument `{name}` at index {index}"
        ))),
        _ => Err(tune_host::HostCallError::new(format!(
            "expected [String] for `{name}`"
        ))),
    }
}
