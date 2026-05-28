use tune_diagnostics::render;

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
