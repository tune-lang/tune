use tune_host::{HostFunction, HostModule, HostParam, HostResourceType};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "fs",
        vec![
            HostFunction::new(
                "read_text",
                vec![HostParam::new("path", Shape::String)],
                string_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.read".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                match std::fs::read_to_string(path) {
                    Ok(text) => Ok(crate::result_ok(Value::String(text))),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "write_text",
                vec![
                    HostParam::new("path", Shape::String),
                    HostParam::new("text", Shape::String),
                ],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("fs.write".into())])
            .with_executor(|args: &[Value]| {
                let path = crate::string_arg(args, 0, "path")?;
                let text = crate::string_arg(args, 1, "text")?;
                match std::fs::write(path, text) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
        ],
    )
    .with_resources(vec![
        HostResourceType::new("File", Shape::Struct("fs.File".into()))
            .with_authorities(vec![
                tune_host::Authority("fs.read".into()),
                tune_host::Authority("fs.write".into()),
            ])
            .retention(tune_host::ResourceRetention::HostRetained)
            .cleanup(tune_host::ResourceCleanup::HostCallback),
    ])
}

fn string_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::String),
        err: Box::new(Shape::String),
    }
}

fn unit_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Unit),
        err: Box::new(Shape::String),
    }
}
