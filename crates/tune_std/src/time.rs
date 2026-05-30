use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tune_host::{Authority, HostFunction, HostModule, HostParam};
use tune_runtime::Value;
use tune_shape::Shape;

#[must_use]
pub fn install() -> HostModule {
    HostModule::new(
        "time",
        vec![
            HostFunction::new("now_millis", Vec::new(), int_result_shape())
                .task_safe(true)
                .with_authorities(vec![time_read_authority()])
                .with_executor(
                    |_: &[Value]| match SystemTime::now().duration_since(UNIX_EPOCH) {
                        Ok(duration) => match i64::try_from(duration.as_millis()) {
                            Ok(value) => Ok(crate::result_ok(Value::Int(value))),
                            Err(_) => Ok(crate::result_error(
                                "current time does not fit in Int milliseconds",
                            )),
                        },
                        Err(error) => Ok(crate::result_error(error.to_string())),
                    },
                ),
            HostFunction::new("monotonic_millis", Vec::new(), Shape::Size)
                .task_safe(true)
                .with_authorities(vec![time_read_authority()])
                .with_executor(|_: &[Value]| {
                    let elapsed = monotonic_start().elapsed();
                    Ok(Value::Size(
                        elapsed.as_millis().try_into().unwrap_or(u64::MAX),
                    ))
                }),
            HostFunction::new(
                "sleep_millis",
                vec![HostParam::new("duration", Shape::Size)],
                unit_result_shape(),
            )
            .task_safe(true)
            .with_authorities(vec![time_sleep_authority()])
            .with_executor(|args: &[Value]| {
                let duration = size_arg(args, 0, "duration")?;
                std::thread::sleep(Duration::from_millis(duration));
                Ok(crate::result_ok(Value::Unit))
            }),
        ],
    )
}

fn monotonic_start() -> &'static Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now)
}

fn time_read_authority() -> Authority {
    Authority("time.read".into())
}

fn time_sleep_authority() -> Authority {
    Authority("time.sleep".into())
}

fn int_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Int),
        err: Box::new(Shape::String),
    }
}

fn unit_result_shape() -> Shape {
    Shape::Result {
        ok: Box::new(Shape::Unit),
        err: Box::new(Shape::String),
    }
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
