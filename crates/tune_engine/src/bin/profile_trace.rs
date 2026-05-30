use std::time::Duration;

use tune_engine::Tune;

#[path = "profile_trace/compare.rs"]
mod compare;

use compare::{
    CompareConfig, compare_profile_rows, parse_csv_rows, print_compare_report, write_csv,
};

#[derive(Debug, Default)]
struct TraceInput {
    paths: Vec<String>,
    full_pipeline: bool,
    csv: bool,
    strict: bool,
    emit_baseline: Option<String>,
    compare: Option<CompareConfig>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProfileRow {
    pub(crate) path: String,
    pub(crate) mode: String,
    pub(crate) stage: String,
    pub(crate) duration_ns: u128,
    pub(crate) plan_ops: usize,
    pub(crate) dynamic_bound_calls: usize,
    pub(crate) ir_ops: usize,
    pub(crate) ir_shape_holes: usize,
    pub(crate) sequence_build_holes: usize,
    pub(crate) bytecode_instructions: usize,
    pub(crate) runtime_type_guard_pressure: usize,
    pub(crate) checked_sequence_ops: usize,
    pub(crate) unchecked_sequence_ops: usize,
    pub(crate) bound_calls: usize,
    pub(crate) diagnostics: usize,
}

fn parse_args() -> Result<TraceInput, String> {
    let mut input = TraceInput::default();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let mut compare = CompareConfig::default();
    let mut compare_seen = false;
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_str();
        match arg {
            "--full" => {
                input.full_pipeline = true;
                i += 1;
            }
            "--csv" => {
                input.csv = true;
                i += 1;
            }
            "--strict-shapes" => {
                input.strict = true;
                i += 1;
            }
            "--emit-baseline" => {
                let baseline = args
                    .get(i + 1)
                    .ok_or("--emit-baseline requires a file path")?
                    .to_owned();
                input.emit_baseline = Some(baseline);
                i += 2;
            }
            "--compare" => {
                compare_seen = true;
                compare.baseline = args
                    .get(i + 1)
                    .ok_or("--compare requires a baseline csv path")?
                    .to_owned();
                i += 2;

                while i < args.len() {
                    let Some(next) = args.get(i).map(String::as_str) else {
                        break;
                    };
                    match next {
                        "--max-stage-delta-pct" => {
                            compare.max_stage_delta_pct = Some(parse_percentage(
                                args.get(i + 1)
                                    .ok_or("--max-stage-delta-pct requires a percentage")?,
                            )?);
                            i += 2;
                        }
                        "--max-counter-delta-pct" => {
                            compare.max_counter_delta_pct = Some(parse_percentage(
                                args.get(i + 1)
                                    .ok_or("--max-counter-delta-pct requires a percentage")?,
                            )?);
                            i += 2;
                        }
                        _ => break,
                    }
                }
            }
            "--max-stage-delta-pct" | "--max-counter-delta-pct" => {
                if !compare_seen {
                    return Err(format!("{arg} must be passed after --compare"));
                }
                let value = args
                    .get(i + 1)
                    .ok_or("--max-counter-delta-pct requires a percentage")?;
                let parsed = parse_percentage(value)?;
                if arg == "--max-stage-delta-pct" {
                    compare.max_stage_delta_pct = Some(parsed);
                } else {
                    compare.max_counter_delta_pct = Some(parsed);
                }
                i += 2;
            }
            arg if arg.starts_with('-') => return Err(format!("unknown option: {arg}")),
            arg => {
                input.paths.push(arg.to_owned());
                i += 1;
            }
        }
    }

    if input.paths.is_empty() {
        return Err("profile_trace needs at least one Tune source path\n\
             usage: cargo run --package tune_engine --bin profile_trace [options] <source.tn>...\n\
             options: --full --csv --strict-shapes\n\
             --emit-baseline <file>\n\
             --compare <baseline.csv> [--max-stage-delta-pct N] [--max-counter-delta-pct N]"
            .to_owned());
    }

    if compare.baseline.is_empty() {
        input.compare = None;
    } else {
        input.compare = Some(compare);
    }

    Ok(input)
}

fn parse_percentage(value: &str) -> Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| format!("invalid percentage: {value}"))?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err(format!("invalid percentage: {value}"));
    }
    Ok(parsed)
}

fn run(input: TraceInput) -> i32 {
    let mut failed = false;
    let mut rows = Vec::new();
    let mode = if input.full_pipeline {
        "full"
    } else {
        "frontend"
    };

    if input.csv {
        println!(
            "path,mode,stage,duration_ns,plan_ops,dynamic_bound_calls,ir_ops,ir_shape_holes,sequence_build_holes,bytecode_instructions,runtime_type_guard_pressure,checked_sequence_ops,unchecked_sequence_ops,bound_calls,diagnostics"
        );
    }

    for path in input.paths {
        let mut tune = Tune::new().with_std();
        let file = match tune.add_path(&path) {
            Ok(file) => file,
            Err(error) => {
                eprintln!("{path}: {error:?}");
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

        if input.csv {
            for stage in &report.timings {
                let row = ProfileRow {
                    path: path.clone(),
                    mode: mode.to_owned(),
                    stage: stage.stage.to_owned(),
                    duration_ns: stage.duration.as_nanos(),
                    plan_ops: report.plan.ops,
                    dynamic_bound_calls: report.plan.dynamic_bound_calls,
                    ir_ops: report.ir.ops,
                    ir_shape_holes: report.ir.shape_holes,
                    sequence_build_holes: report.ir.sequence_build_holes,
                    bytecode_instructions: report.bytecode.instructions,
                    runtime_type_guard_pressure: report.bytecode.runtime_type_guard_pressure,
                    checked_sequence_ops: report.bytecode.checked_sequence_ops,
                    unchecked_sequence_ops: report.bytecode.unchecked_sequence_ops,
                    bound_calls: report.bytecode.bound_calls,
                    diagnostics: report.diagnostics.len(),
                };
                println!(
                    "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                    row.path,
                    row.mode,
                    row.stage,
                    row.duration_ns,
                    row.plan_ops,
                    row.dynamic_bound_calls,
                    row.ir_ops,
                    row.ir_shape_holes,
                    row.sequence_build_holes,
                    row.bytecode_instructions,
                    row.runtime_type_guard_pressure,
                    row.checked_sequence_ops,
                    row.unchecked_sequence_ops,
                    row.bound_calls,
                    row.diagnostics
                );
                rows.push(row);
            }
            continue;
        }

        println!("{path}");
        let total_nanos = report
            .timings
            .iter()
            .map(|timing| timing.duration.as_nanos())
            .sum::<u128>();

        for timing in &report.timings {
            let share = if total_nanos == 0 {
                0.0
            } else {
                (timing.duration.as_nanos() as f64) * 100.0 / (total_nanos as f64)
            };
            println!(
                "  {}: {} ({share:.2}%)",
                timing.stage,
                format_duration(timing.duration),
            );
        }

        println!(
            "  ops: plan={} ir={} optimizer_changed_passes={} bytecode_instructions={}",
            report.plan.ops,
            report.ir.ops,
            report.optimizer.changed_passes,
            report.bytecode.instructions,
        );
        println!(
            "  holes: ir_shape_holes={} ir_sequence_build_holes={}",
            report.ir.shape_holes, report.ir.sequence_build_holes
        );
        println!(
            "  guards: runtime={} plan_bound_calls={} bytecode_bound_calls={} seq_checked={} seq_unchecked={}",
            report.bytecode.runtime_type_guard_pressure,
            report.plan.dynamic_bound_calls,
            report.bytecode.bound_calls,
            report.bytecode.checked_sequence_ops,
            report.bytecode.unchecked_sequence_ops,
        );

        if input.strict && report.ir.shape_holes != 0 {
            eprintln!(
                "{path}: expected zero ir shape holes, observed {}",
                report.ir.shape_holes
            );
            failed = true;
        }
        if !report.diagnostics.is_empty() {
            eprintln!("{path}: diagnostics={}", report.diagnostics.len());
            failed = true;
        }

        for stage in &report.timings {
            rows.push(ProfileRow {
                path: path.clone(),
                mode: mode.to_owned(),
                stage: stage.stage.to_owned(),
                duration_ns: stage.duration.as_nanos(),
                plan_ops: report.plan.ops,
                dynamic_bound_calls: report.plan.dynamic_bound_calls,
                ir_ops: report.ir.ops,
                ir_shape_holes: report.ir.shape_holes,
                sequence_build_holes: report.ir.sequence_build_holes,
                bytecode_instructions: report.bytecode.instructions,
                runtime_type_guard_pressure: report.bytecode.runtime_type_guard_pressure,
                checked_sequence_ops: report.bytecode.checked_sequence_ops,
                unchecked_sequence_ops: report.bytecode.unchecked_sequence_ops,
                bound_calls: report.bytecode.bound_calls,
                diagnostics: report.diagnostics.len(),
            });
        }
    }

    if let Some(path) = input.emit_baseline.as_deref()
        && let Err(error) = write_csv(path, &rows)
    {
        eprintln!("failed to write baseline {path}: {error}");
        return 1;
    }

    if let Some(compare) = input.compare.as_ref() {
        let baseline_rows = match parse_csv_rows(&compare.baseline) {
            Ok(rows) => rows,
            Err(error) => {
                eprintln!("failed to read baseline {}: {error}", compare.baseline);
                return 1;
            }
        };
        let report = compare_profile_rows(&baseline_rows, &rows, compare);
        print_compare_report(&report, compare);
        if report.total_regressions > 0
            || report.missing_in_current > 0
            || report.extra_in_current > 0
        {
            failed = true;
        }
    }

    i32::from(failed)
}

fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    if nanos >= 1_000_000_000 {
        format!("{:.3}s", (nanos as f64) / 1_000_000_000.0)
    } else if nanos >= 1_000_000 {
        format!("{:.3}ms", (nanos as f64) / 1_000_000.0)
    } else if nanos >= 1_000 {
        format!("{:.3}µs", (nanos as f64) / 1_000.0)
    } else {
        format!("{nanos}ns")
    }
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
