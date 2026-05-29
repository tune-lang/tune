pub mod fs;
pub mod io;
pub mod json;
pub mod meta;
pub mod parse;
pub mod prelude;

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
    vec![io::install(), parse::install(), fs::install()]
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
        _ => Err(tune_host::HostCallError::new(format!(
            "expected String for `{name}`"
        ))),
    }
}
