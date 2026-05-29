use tune_diagnostics::render;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Build { path: Option<String> },
    Check { path: Option<String> },
    Run { path: Option<String> },
    Profile { path: Option<String> },
    New { name: String },
    Help,
}

pub fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Help),
        [flag] if flag == "-h" || flag == "--help" => Ok(CliCommand::Help),
        [command] if command == "build" => Ok(CliCommand::Build { path: None }),
        [command] if command == "run" => Ok(CliCommand::Run { path: None }),
        [command] if command == "check" => Ok(CliCommand::Check { path: None }),
        [command] if command == "profile" => Ok(CliCommand::Profile { path: None }),
        [path] => Ok(CliCommand::Run {
            path: Some(path.clone()),
        }),
        [command, path] if command == "run" => Ok(CliCommand::Run {
            path: Some(path.clone()),
        }),
        [command, path] if command == "check" => Ok(CliCommand::Check {
            path: Some(path.clone()),
        }),
        [command, path] if command == "build" => Ok(CliCommand::Build {
            path: Some(path.clone()),
        }),
        [command, path] if command == "profile" => Ok(CliCommand::Profile {
            path: Some(path.clone()),
        }),
        [command, name] if command == "new" => Ok(CliCommand::New { name: name.clone() }),
        [command, ..] => Err(format!("unknown dyno command `{command}`")),
    }
}

#[must_use]
pub fn usage() -> &'static str {
    "usage: dyno new <name>\n       dyno check [file]\n       dyno run [file]\n       dyno build [file]\n       dyno profile [file]\n       dyno <file>"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProject {
    pub name: String,
    pub root: std::path::PathBuf,
    pub manifest: std::path::PathBuf,
    pub entry: std::path::PathBuf,
}

pub fn create_project(name: &str) -> Result<NewProject, String> {
    create_project_in(".", name)
}

pub fn create_project_in(
    base: impl AsRef<std::path::Path>,
    name: &str,
) -> Result<NewProject, String> {
    validate_project_name(name)?;
    let root = base.as_ref().join(name);
    if root.exists() {
        return Err(format!("project path `{name}` already exists"));
    }
    let src = root.join("src");
    std::fs::create_dir_all(&src).map_err(|error| format!("failed to create {src:?}: {error}"))?;

    let manifest = root.join("dyno.toml");
    let entry = src.join("main.tn");
    let project_manifest = dyno_project::Manifest::new(name, "src/main.tn");
    std::fs::write(&manifest, project_manifest.to_toml())
        .map_err(|error| format!("failed to write {manifest:?}: {error}"))?;
    std::fs::write(&entry, entry_template())
        .map_err(|error| format!("failed to write {entry:?}: {error}"))?;

    Ok(NewProject {
        name: name.to_owned(),
        root,
        manifest,
        entry,
    })
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("project name must not be empty".to_owned());
    }
    if name == "." || name == ".." || name.contains(std::path::MAIN_SEPARATOR) {
        return Err(format!("invalid project name `{name}`"));
    }
    if !name
        .chars()
        .all(|item| item.is_ascii_alphanumeric() || item == '_' || item == '-')
    {
        return Err(format!(
            "project name `{name}` may only contain ASCII letters, numbers, `_`, and `-`"
        ));
    }
    Ok(())
}

fn entry_template() -> &'static str {
    "let message: String = \"hello from Dyno\"\n"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub manifest: dyno_project::Manifest,
    pub sources: Vec<(String, String)>,
}

pub fn load_project_from_dir(root: impl AsRef<std::path::Path>) -> Result<LoadedProject, String> {
    let loaded = dyno_project::load_project_dir(root).map_err(|error| format!("{error:?}"))?;
    Ok(LoadedProject {
        manifest: loaded.manifest,
        sources: loaded.sources,
    })
}

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

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
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
        .map(render::render_plain)
        .collect()
}
