#[derive(Debug, Clone)]
pub struct PropagationFrame {
    pub function: String,
    pub expression: String,
    pub file: String,
    pub line: u32,
}
