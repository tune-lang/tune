use crate::function::BytecodeFunction;

#[derive(Debug, Clone)]
pub struct BytecodeArtifact {
    pub functions: Vec<BytecodeFunction>,
    pub constants: Vec<String>,
}
