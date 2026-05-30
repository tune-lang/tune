use std::collections::HashMap;

use crate::ProfileRow;

#[derive(Debug, Default)]
pub(crate) struct CompareConfig {
    pub(crate) baseline: String,
    pub(crate) max_stage_delta_pct: Option<f64>,
    pub(crate) max_counter_delta_pct: Option<f64>,
}

#[derive(Debug)]
pub(crate) struct CompareReport {
    pub(crate) total_diffs: usize,
    pub(crate) total_regressions: usize,
    pub(crate) missing_in_current: usize,
    pub(crate) extra_in_current: usize,
    diffs: Vec<CompareDiff>,
}

#[derive(Debug)]
struct CompareDiff {
    path: String,
    mode: String,
    stage: String,
    metric: String,
    baseline: u128,
    current: u128,
    delta_pct: Option<f64>,
    is_regression: bool,
}

pub(crate) fn compare_profile_rows(
    baseline: &[ProfileRow],
    current: &[ProfileRow],
    compare: &CompareConfig,
) -> CompareReport {
    let mut baseline_map: HashMap<(String, String, String), ProfileRow> = HashMap::new();
    for row in baseline {
        baseline_map.insert(
            (row.path.clone(), row.mode.clone(), row.stage.clone()),
            row.clone(),
        );
    }

    let mut report = CompareReport {
        total_diffs: 0,
        total_regressions: 0,
        missing_in_current: 0,
        extra_in_current: 0,
        diffs: Vec::new(),
    };

    for row in current {
        let key = (row.path.clone(), row.mode.clone(), row.stage.clone());
        let Some(expected) = baseline_map.remove(&key) else {
            report.extra_in_current += 1;
            report.total_diffs += 1;
            continue;
        };
        compare_metric(
            &mut report,
            row,
            "duration_ns",
            expected.duration_ns as f64,
            row.duration_ns as f64,
            compare.max_stage_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "plan_ops",
            expected.plan_ops as f64,
            row.plan_ops as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "ir_ops",
            expected.ir_ops as f64,
            row.ir_ops as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "dynamic_bound_calls",
            expected.dynamic_bound_calls as f64,
            row.dynamic_bound_calls as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "ir_shape_holes",
            expected.ir_shape_holes as f64,
            row.ir_shape_holes as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "sequence_build_holes",
            expected.sequence_build_holes as f64,
            row.sequence_build_holes as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "bytecode_instructions",
            expected.bytecode_instructions as f64,
            row.bytecode_instructions as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "runtime_type_guard_pressure",
            expected.runtime_type_guard_pressure as f64,
            row.runtime_type_guard_pressure as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "checked_sequence_ops",
            expected.checked_sequence_ops as f64,
            row.checked_sequence_ops as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "unchecked_sequence_ops",
            expected.unchecked_sequence_ops as f64,
            row.unchecked_sequence_ops as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "bound_calls",
            expected.bound_calls as f64,
            row.bound_calls as f64,
            compare.max_counter_delta_pct,
        );
        compare_metric(
            &mut report,
            row,
            "diagnostics",
            expected.diagnostics as f64,
            row.diagnostics as f64,
            compare.max_counter_delta_pct,
        );
    }

    if !baseline_map.is_empty() {
        report.missing_in_current += baseline_map.len();
        report.total_diffs += baseline_map.len();
    }

    report
}

fn compare_metric(
    report: &mut CompareReport,
    current: &ProfileRow,
    metric: &str,
    expected_value: f64,
    current_value: f64,
    max_delta_pct: Option<f64>,
) {
    if expected_value == current_value {
        return;
    }

    let delta = current_value - expected_value;
    let delta_pct = if expected_value == 0.0 {
        if current_value == 0.0 {
            Some(0.0)
        } else {
            None
        }
    } else {
        Some((delta * 100.0) / expected_value)
    };

    let is_regression = if delta <= 0.0 {
        false
    } else if let Some(limit) = max_delta_pct {
        match delta_pct {
            Some(pct) => pct.abs() > limit,
            None => true,
        }
    } else {
        false
    };

    report.total_diffs += 1;
    if is_regression {
        report.total_regressions += 1;
    }

    report.diffs.push(CompareDiff {
        path: current.path.clone(),
        mode: current.mode.clone(),
        stage: current.stage.clone(),
        metric: metric.to_owned(),
        baseline: expected_value as u128,
        current: current_value as u128,
        delta_pct,
        is_regression,
    });
}

pub(crate) fn print_compare_report(report: &CompareReport, compare: &CompareConfig) {
    if report.total_diffs == 0 && report.missing_in_current == 0 && report.extra_in_current == 0 {
        println!(
            "compare={}: no deltas (stage_limit={:?} counter_limit={:?})",
            compare.baseline, compare.max_stage_delta_pct, compare.max_counter_delta_pct
        );
        return;
    }

    println!(
        "compare={}: diffs={} regressions={} missing_current={} missing_baseline={}",
        compare.baseline,
        report.total_diffs,
        report.total_regressions,
        report.missing_in_current,
        report.extra_in_current
    );

    if report.extra_in_current > 0 {
        println!(
            "  current has extra sample rows not in baseline: {count}",
            count = report.extra_in_current
        );
    }
    if report.missing_in_current > 0 {
        println!(
            "  missing sample rows in current: {count}",
            count = report.missing_in_current
        );
    }

    for diff in &report.diffs {
        match diff.delta_pct {
            Some(pct) => {
                println!(
                    "  {} {} {} {}: {} -> {} ({pct:+.2}%){}",
                    diff.path,
                    diff.mode,
                    diff.stage,
                    diff.metric,
                    diff.baseline,
                    diff.current,
                    if diff.is_regression {
                        " [regression threshold]"
                    } else {
                        ""
                    },
                );
            }
            None => {
                println!(
                    "  {} {} {} {}: {} -> {} (inf)",
                    diff.path, diff.mode, diff.stage, diff.metric, diff.baseline, diff.current,
                );
            }
        }
    }
}

pub(crate) fn parse_csv_rows(path: &str) -> Result<Vec<ProfileRow>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to open baseline csv {path}: {error}"))?;
    let mut rows = Vec::new();

    for (index, line) in text.lines().enumerate() {
        if index == 0 {
            continue;
        }
        let parts = line.split(',').collect::<Vec<_>>();
        if parts.is_empty() || parts[0].trim().is_empty() {
            continue;
        }
        if parts.len() != 10 && parts.len() != 15 {
            return Err(format!(
                "invalid baseline csv row: expected 10 or 15 fields, found {}",
                parts.len()
            ));
        }
        rows.push(if parts.len() == 15 {
            ProfileRow {
                path: parts[0].to_owned(),
                mode: parts[1].to_owned(),
                stage: parts[2].to_owned(),
                duration_ns: parse_u128(parts[3])?,
                plan_ops: parse_usize(parts[4])?,
                dynamic_bound_calls: parse_usize(parts[5])?,
                ir_ops: parse_usize(parts[6])?,
                ir_shape_holes: parse_usize(parts[7])?,
                sequence_build_holes: parse_usize(parts[8])?,
                bytecode_instructions: parse_usize(parts[9])?,
                runtime_type_guard_pressure: parse_usize(parts[10])?,
                checked_sequence_ops: parse_usize(parts[11])?,
                unchecked_sequence_ops: parse_usize(parts[12])?,
                bound_calls: parse_usize(parts[13])?,
                diagnostics: parse_usize(parts[14])?,
            }
        } else {
            ProfileRow {
                path: parts[0].to_owned(),
                mode: parts[1].to_owned(),
                stage: parts[2].to_owned(),
                duration_ns: parse_u128(parts[3])?,
                plan_ops: parse_usize(parts[4])?,
                dynamic_bound_calls: 0,
                ir_ops: parse_usize(parts[5])?,
                ir_shape_holes: parse_usize(parts[6])?,
                sequence_build_holes: parse_usize(parts[7])?,
                bytecode_instructions: parse_usize(parts[8])?,
                runtime_type_guard_pressure: 0,
                checked_sequence_ops: 0,
                unchecked_sequence_ops: 0,
                bound_calls: 0,
                diagnostics: parse_usize(parts[9])?,
            }
        });
    }

    if rows.is_empty() {
        return Err("baseline csv has no rows".to_owned());
    }
    Ok(rows)
}

pub(crate) fn write_csv(path: &str, rows: &[ProfileRow]) -> Result<(), String> {
    let mut lines = Vec::new();
    lines.push(
        "path,mode,stage,duration_ns,plan_ops,dynamic_bound_calls,ir_ops,ir_shape_holes,sequence_build_holes,bytecode_instructions,runtime_type_guard_pressure,checked_sequence_ops,unchecked_sequence_ops,bound_calls,diagnostics"
            .to_owned(),
    );
    for row in rows {
        lines.push(format!(
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
        ));
    }

    std::fs::write(path, lines.join("\n"))
        .map_err(|error| format!("failed to write baseline {path}: {error}"))
}

fn parse_u128(value: &str) -> Result<u128, String> {
    value
        .parse::<u128>()
        .map_err(|_| format!("invalid integer in baseline csv: {value}"))
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid integer in baseline csv: {value}"))
}
