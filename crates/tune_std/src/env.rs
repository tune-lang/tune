use tune_host::{Authority, HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "env",
        vec![
            HostFunction::new("args", Vec::new(), Shape::Sequence(Box::new(Shape::String)))
                .with_authorities(vec![env_read_authority()])
                .with_executor(|_: &[Value]| {
                    Ok(Value::Sequence(
                        std::env::args().map(Value::String).collect::<Vec<_>>(),
                    ))
                }),
            HostFunction::new(
                "get",
                vec![HostParam::new("name", Shape::String)],
                Shape::Optional(Box::new(Shape::String)),
            )
            .with_authorities(vec![env_read_authority()])
            .with_executor(|args: &[Value]| {
                let name = crate::string_arg(args, 0, "name")?;
                match std::env::var(name) {
                    Ok(value) => Ok(Value::String(value)),
                    Err(std::env::VarError::NotPresent) => Ok(Value::None),
                    Err(std::env::VarError::NotUnicode(_)) => Err(tune_host::HostCallError::new(
                        format!("environment variable `{name}` is not valid Unicode"),
                    )),
                }
            }),
            HostFunction::new("cwd", Vec::new(), string_result_shape())
                .with_authorities(vec![env_read_authority()])
                .with_executor(|_: &[Value]| match std::env::current_dir() {
                    Ok(path) => Ok(crate::result_ok(Value::String(
                        path.to_string_lossy().to_string(),
                    ))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }),
        ],
    )
}

fn env_read_authority() -> Authority {
    Authority("env.read".into())
}

fn string_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::String),
        err: Box::new(Shape::String),
    }
}
