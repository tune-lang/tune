use std::io::Write;

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
            HostFunction::new(
                "write",
                vec![HostParam::new("text", Shape::String)],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("io.write".into())])
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let mut stdout = std::io::stdout().lock();
                match stdout
                    .write_all(text.as_bytes())
                    .and_then(|()| stdout.flush())
                {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "write_line",
                vec![HostParam::new("text", Shape::String)],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("io.write".into())])
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let mut stdout = std::io::stdout().lock();
                match writeln!(stdout, "{text}").and_then(|()| stdout.flush()) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "error_line",
                vec![HostParam::new("text", Shape::String)],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("io.error".into())])
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let mut stderr = std::io::stderr().lock();
                match writeln!(stderr, "{text}").and_then(|()| stderr.flush()) {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new(
                "error",
                vec![HostParam::new("text", Shape::String)],
                unit_result_shape(),
            )
            .with_authorities(vec![tune_host::Authority("io.error".into())])
            .with_executor(|args: &[Value]| {
                let text = crate::string_arg(args, 0, "text")?;
                let mut stderr = std::io::stderr().lock();
                match stderr
                    .write_all(text.as_bytes())
                    .and_then(|()| stderr.flush())
                {
                    Ok(()) => Ok(crate::result_ok(Value::Unit)),
                    Err(error) => Ok(crate::result_error(error.to_string())),
                }
            }),
            HostFunction::new("flush", Vec::new(), unit_result_shape())
                .with_authorities(vec![tune_host::Authority("io.write".into())])
                .with_executor(|_: &[Value]| {
                    let mut stdout = std::io::stdout().lock();
                    match stdout.flush() {
                        Ok(()) => Ok(crate::result_ok(Value::Unit)),
                        Err(error) => Ok(crate::result_error(error.to_string())),
                    }
                }),
            HostFunction::new("read_line", Vec::new(), string_result_shape())
                .with_authorities(vec![tune_host::Authority("io.read".into())])
                .with_executor(|_: &[Value]| {
                    let mut line = String::new();
                    match std::io::stdin().read_line(&mut line) {
                        Ok(_) => {
                            if line.ends_with('\n') {
                                line.pop();
                                if line.ends_with('\r') {
                                    line.pop();
                                }
                            }
                            Ok(crate::result_ok(Value::String(line)))
                        }
                        Err(error) => Ok(crate::result_error(error.to_string())),
                    }
                }),
        ],
    )
}

fn unit_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Unit),
        err: Box::new(Shape::String),
    }
}

fn string_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::String),
        err: Box::new(Shape::String),
    }
}
