use tune_runtime::Value;

#[test]
fn inline_no_arg_host_float_call_flows_through_binary_ops() -> Result<(), &'static str> {
    let mut tune = tune_engine::Tune::new().with_std();
    let file = tune
        .add_file(
            "main.tn",
            r#"
import "math"
let radius: Float = 2.0
let result: Float = math.pow(radius, 2.0) * math.pi()
"#,
        )
        .ok_or("file should allocate")?;

    let check = tune.check_file(file).ok_or("file should check")?;
    if !check.diagnostics.is_empty() {
        eprintln!("{:?}", check.diagnostics);
        return Err("inline host Float call should check");
    }

    let value = tune.run_file(file).map_err(|error| {
        eprintln!("{error:?}");
        "inline host Float call should execute"
    })?;
    let Value::Float(value) = value else {
        return Err("result should be Float");
    };
    assert!((value - (4.0 * std::f64::consts::PI)).abs() < f64::EPSILON);
    Ok(())
}
