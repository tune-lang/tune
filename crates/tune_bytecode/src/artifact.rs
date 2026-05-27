use crate::function::BytecodeFunction;

#[derive(Debug, Clone)]
pub struct BytecodeArtifact {
    pub entry_function: Option<u32>,
    pub functions: Vec<BytecodeFunction>,
    pub constants: Vec<String>,
}
