use tune_host::{HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "io",
        vec![
            HostFunction::new(
                "print",
                vec![HostParam::new("message", Shape::String)],
                Shape::Unit,
            )
            .with_authorities(vec![tune_host::Authority("io.write".into())])
            .with_executor(|args: &[Value]| {
                let message = crate::string_arg(args, 0, "message")?;
                println!("{message}");
                Ok(Value::Unit)
            }),
        ],
    )
}
