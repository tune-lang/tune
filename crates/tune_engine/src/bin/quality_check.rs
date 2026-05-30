use tune_diagnostics::render::render_plain;
use tune_engine::Tune;

#[derive(Debug, Default)]
struct QualityInput {
    paths: Vec<String>,
    full_pipeline: bool,
    require_zero_shape_holes: bool,
    allow_frontend_warnings: bool,
    max_runtime_guards: Option<usize>,
    max_dynamic_bound_calls: Option<usize>,
    max_checked_sequence_ops: Option<usize>,
}

fn parse_args() -> Result<QualityInput, String> {
    let mut input = QualityInput::default();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--full" => {
                input.full_pipeline = true;
                index += 1;
            }
            "--strict-shapes" => {
                input.require_zero_shape_holes = true;
                index += 1;
            }
            "--allow-warnings" => {
                input.allow_frontend_warnings = true;
                index += 1;
            }
            "--max-runtime-guards" => {
                input.max_runtime_guards = Some(parse_usize_arg(&args, index, arg)?);
                index += 2;
            }
            "--max-dynamic-bound-calls" => {
                input.max_dynamic_bound_calls = Some(parse_usize_arg(&args, index, arg)?);
                index += 2;
            }
            "--max-checked-sequence-ops" => {
                input.max_checked_sequence_ops = Some(parse_usize_arg(&args, index, arg)?);
                index += 2;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }
            arg => {
                input.paths.push(arg.to_owned());
                index += 1;
            }
        }
    }
    if input.paths.is_empty() {
        return Err("quality_check needs at least one Tune source path\n\
             usage: cargo run -p tune_engine --bin quality_check [--full] [--strict-shapes]\n\
             [--allow-warnings] [--max-runtime-guards N]\n\
             [--max-dynamic-bound-calls N] [--max-checked-sequence-ops N]\n\
             <source.tn>..."
            .to_owned());
    }
    Ok(input)
}

fn parse_usize_arg(args: &[String], index: usize, flag: &str) -> Result<usize, String> {
    let value = args
        .get(index + 1)
        .ok_or_else(|| format!("{flag} requires an integer value"))?;
    if value.starts_with('-') {
        return Err(format!("{flag} requires an integer value"));
    }
    value
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires an integer value, got `{value}`"))
}

fn run(input: QualityInput) -> i32 {
    let mut failed = false;

    for path in input.paths {
        let mut tune = Tune::new().with_std();
        let file = match tune.add_path(&path) {
            Ok(file) => file,
            Err(error) => {
                eprintln!("{}: {error:?}", path);
                failed = true;
                continue;
            }
        };

        let report = match if input.full_pipeline {
            tune.profile_file(file)
        } else {
            tune.profile_file_frontend(file)
        } {
            Ok(report) => report,
            Err(error) => {
                eprintln!("{path}: profile failed: {error:?}");
                failed = true;
                continue;
            }
        };

        println!(
            "{path}: mode={} plan_ops={} dynamic_bound_calls={} ir_ops={} ir_shape_holes={} ir_seq_holes={} optimizer_changes={} bytecode_ops={} runtime_guards={} checked_seq_ops={} unchecked_seq_ops={} bound_calls={}",
            if input.full_pipeline {
                "full"
            } else {
                "frontend"
            },
            report.plan.ops,
            report.plan.dynamic_bound_calls,
            report.ir.ops,
            report.ir.shape_holes,
            report.ir.sequence_build_holes,
            report.optimizer.changed_passes,
            report.bytecode.instructions,
            report.bytecode.runtime_type_guard_pressure,
            report.bytecode.checked_sequence_ops,
            report.bytecode.unchecked_sequence_ops,
            report.bytecode.bound_calls,
        );

        if !report.diagnostics.is_empty() && !input.allow_frontend_warnings {
            println!("{path}: diagnostics {}", report.diagnostics.len());
            for diagnostic in report.diagnostics {
                println!("  {}", render_plain(&diagnostic));
            }
            failed = true;
            continue;
        }

        if input.require_zero_shape_holes && report.ir.shape_holes != 0 {
            println!(
                "{path}: expected zero IR shape holes, observed {}",
                report.ir.shape_holes
            );
            failed = true;
        }
        failed |= check_limit(
            &path,
            "runtime guard pressure",
            report.bytecode.runtime_type_guard_pressure,
            input.max_runtime_guards,
        );
        failed |= check_limit(
            &path,
            "dynamic bound calls",
            report.plan.dynamic_bound_calls,
            input.max_dynamic_bound_calls,
        );
        failed |= check_limit(
            &path,
            "checked sequence ops",
            report.bytecode.checked_sequence_ops,
            input.max_checked_sequence_ops,
        );
    }

    i32::from(failed)
}

fn check_limit(path: &str, label: &str, observed: usize, limit: Option<usize>) -> bool {
    let Some(limit) = limit else {
        return false;
    };
    if observed <= limit {
        return false;
    }
    println!("{path}: expected {label} <= {limit}, observed {observed}");
    true
}

fn main() {
    let input = match parse_args() {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };

    std::process::exit(run(input));
}
