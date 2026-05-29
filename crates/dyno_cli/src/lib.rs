use tune_diagnostics::render;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Check { path: String },
    Run { path: String },
    Help,
}

#[must_use]
pub fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Help),
        [flag] if flag == "-h" || flag == "--help" => Ok(CliCommand::Help),
        [path] => Ok(CliCommand::Run { path: path.clone() }),
        [command, path] if command == "run" => Ok(CliCommand::Run { path: path.clone() }),
        [command, path] if command == "check" => Ok(CliCommand::Check { path: path.clone() }),
        [command, ..] => Err(format!("unknown dyno command `{command}`")),
    }
}

#[must_use]
pub fn usage() -> &'static str {
    "usage: dyno check <file>\n       dyno run <file>\n       dyno <file>"
}

#[must_use]
pub fn render_engine_error(error: &tune_engine::EngineError) -> Vec<String> {
    match error {
        tune_engine::EngineError::Diagnostics(diagnostics) => diagnostics
            .iter()
            .map(render::render_plain)
            .collect::<Vec<_>>(),
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
