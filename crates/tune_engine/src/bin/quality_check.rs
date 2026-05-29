use tune_diagnostics::render::render_plain;
use tune_engine::Tune;

#[derive(Debug, Default)]
struct QualityInput {
    paths: Vec<String>,
    require_zero_shape_holes: bool,
    allow_frontend_warnings: bool,
}

fn parse_args() -> Result<QualityInput, String> {
    let mut input = QualityInput::default();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--strict-shapes" => input.require_zero_shape_holes = true,
            "--allow-warnings" => input.allow_frontend_warnings = true,
            arg if arg.starts_with('-') => {
                return Err(format!("unknown option: {arg}"));
            }
            arg => input.paths.push(arg.to_owned()),
        }
    }
    if input.paths.is_empty() {
        return Err(
            "quality_check needs at least one Tune source path\n\
             usage: cargo run -p tune_engine --bin quality_check [--strict-shapes] [--allow-warnings] <source.tn>..."
                .to_owned(),
        );
    }
    Ok(input)
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

        let report = match tune.profile_file_frontend(file) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("{path}: profile failed: {error:?}");
                failed = true;
                continue;
            }
        };

        println!(
            "{path}: plan_ops={} ir_ops={} ir_shape_holes={} ir_seq_holes={} optimizer_changes={} bytecode_ops={}",
            report.plan.ops,
            report.ir.ops,
            report.ir.shape_holes,
            report.ir.sequence_build_holes,
            report.optimizer.changed_passes,
            report.bytecode.instructions,
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
    }

    i32::from(failed)
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
