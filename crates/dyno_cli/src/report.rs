use tune_diagnostics::render;

#[must_use]
pub fn render_profile_report(report: &tune_engine::ProfileReport) -> String {
    let mut output = String::new();
    push_line(&mut output, "compile stages:");
    for timing in &report.timings {
        push_line(
            &mut output,
            &format!(
                "  {:<12} {:>10.3} ms",
                timing.stage,
                timing.duration.as_secs_f64() * 1000.0
            ),
        );
    }
    push_line(&mut output, "");
    push_line(&mut output, "plan quality:");
    push_line(
        &mut output,
        &format!("  functions: {}", report.plan.functions),
    );
    push_line(&mut output, &format!("  ops: {}", report.plan.ops));
    push_line(
        &mut output,
        &format!("  direct calls: {}", report.plan.direct_calls),
    );
    push_line(
        &mut output,
        &format!("  dynamic bound calls: {}", report.plan.dynamic_bound_calls),
    );
    push_line(
        &mut output,
        &format!(
            "  struct index ops: get={}, set={}",
            report.plan.struct_index_gets, report.plan.struct_index_sets
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  finite for: sequence={}, range={}, member={}, unknown={}",
            report.plan.finite_for_sequence,
            report.plan.finite_for_range,
            report.plan.finite_for_member_access,
            report.plan.finite_for_unknown
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  unresolved/witness/host: {}/{}/{}",
            report.plan.unresolved_member_calls, report.plan.witness_calls, report.plan.host_calls
        ),
    );
    push_line(&mut output, "");
    push_line(&mut output, "ir quality:");
    push_line(
        &mut output,
        &format!("  functions: {}", report.ir.functions),
    );
    push_line(&mut output, &format!("  ops: {}", report.ir.ops));
    push_line(
        &mut output,
        &format!("  shape holes: {}", report.ir.shape_holes),
    );
    push_line(
        &mut output,
        &format!(
            "  sequence builds with holes: {}",
            report.ir.sequence_build_holes
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  sequence ops: checked={}, unchecked={}",
            report.ir.checked_sequence_ops, report.ir.unchecked_sequence_ops
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  generic finite-for ops: {}",
            report.ir.generic_finite_for_ops
        ),
    );
    push_line(&mut output, "");
    push_line(&mut output, "optimizer quality:");
    push_line(
        &mut output,
        &format!("  changed passes: {}", report.optimizer.changed_passes),
    );
    push_line(
        &mut output,
        &format!(
            "  ownership: stack={}, direct_drop={}, rc={}, cow={}, atomic={}, host={}",
            report.optimizer.stack,
            report.optimizer.direct_drop,
            report.optimizer.non_atomic_rc,
            report.optimizer.cow,
            report.optimizer.shared_atomic,
            report.optimizer.host_retained
        ),
    );
    push_line(&mut output, "");
    push_line(&mut output, "bytecode quality:");
    push_line(
        &mut output,
        &format!("  functions: {}", report.bytecode.functions),
    );
    push_line(
        &mut output,
        &format!("  instructions: {}", report.bytecode.instructions),
    );
    push_line(
        &mut output,
        &format!(
            "  registers/locals/constants: {}/{}/{}",
            report.bytecode.registers, report.bytecode.locals, report.bytecode.constants
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  calls: direct={}, bound={}, callable_values={}",
            report.bytecode.direct_calls,
            report.bytecode.bound_calls,
            report.bytecode.callable_values
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  sequence ops: checked={}, unchecked={}",
            report.bytecode.checked_sequence_ops, report.bytecode.unchecked_sequence_ops
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  field/variant field accesses: {}/{}",
            report.bytecode.field_accesses, report.bytecode.variant_field_accesses
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  runtime guard pressure: {}",
            report.bytecode.runtime_type_guard_pressure
        ),
    );
    push_line(
        &mut output,
        &format!(
            "  unsupported reserved opcodes: {}",
            report.bytecode.unsupported_reserved_opcodes
        ),
    );
    if !report.bytecode.opcodes.is_empty() {
        push_line(&mut output, "  opcodes:");
        for opcode in &report.bytecode.opcodes {
            push_line(
                &mut output,
                &format!("    {:?}: {}", opcode.opcode, opcode.count),
            );
        }
    }
    if let Some(reason) = &report.stop_reason {
        push_line(&mut output, "");
        push_line(&mut output, &format!("stopped: {reason}"));
    }
    if !report.diagnostics.is_empty() {
        push_line(&mut output, "");
        push_line(
            &mut output,
            &format!("diagnostics: {}", report.diagnostics.len()),
        );
    }
    output
}

#[must_use]
pub fn render_build_report(report: &tune_engine::ExecutableReport) -> String {
    let functions = report.bytecode.functions.len();
    let instructions = report
        .bytecode
        .functions
        .iter()
        .map(|function| function.instructions.len())
        .sum::<usize>();
    let constants = report.bytecode.constants.len();
    format!(
        "built executable: functions={functions}, instructions={instructions}, constants={constants}"
    )
}

#[must_use]
pub fn render_engine_error(error: &tune_engine::EngineError) -> Vec<String> {
    match error {
        tune_engine::EngineError::Diagnostics(diagnostics) => diagnostics
            .iter()
            .map(render::render_plain)
            .collect::<Vec<_>>(),
        tune_engine::EngineError::ProjectLoad(message) => vec![message.clone()],
        tune_engine::EngineError::SourceLoad(message) => vec![message.clone()],
        _ => vec![format!("execution failed: {error:?}")],
    }
}

#[must_use]
pub fn render_engine_error_with_sources(
    error: &tune_engine::EngineError,
    db: &tune_db::TuneDb,
) -> Vec<String> {
    match error {
        tune_engine::EngineError::Diagnostics(diagnostics) => diagnostics
            .iter()
            .map(|diagnostic| render::render_plain_with_sources(diagnostic, db))
            .collect::<Vec<_>>(),
        tune_engine::EngineError::ProjectLoad(message) => vec![message.clone()],
        tune_engine::EngineError::SourceLoad(message) => vec![message.clone()],
        _ => vec![format!("execution failed: {error:?}")],
    }
}

#[must_use]
pub fn render_runtime_boundary(value: &tune_runtime::Value) -> Vec<String> {
    tune_engine::diagnostics_from_runtime_value(value)
        .iter()
        .map(render::render_plain)
        .collect()
}

#[must_use]
pub fn render_runtime_boundary_with_sources(
    value: &tune_runtime::Value,
    db: &tune_db::TuneDb,
) -> Vec<String> {
    tune_engine::diagnostics_from_runtime_value_with_sources(value, db)
        .iter()
        .map(|diagnostic| render::render_plain_with_sources(diagnostic, db))
        .collect()
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}
