use crate::function::{BytecodeFunction, BytecodeStructLayout};
use tune_diagnostics::Span;

#[derive(Debug, Clone)]
pub struct BytecodeArtifact {
    pub entry_function: Option<u32>,
    pub functions: Vec<BytecodeFunction>,
    pub struct_layouts: Vec<BytecodeStructLayout>,
    pub constants: Vec<BytecodeConst>,
}

impl BytecodeArtifact {
    #[must_use]
    pub fn function_span(&self, function: u32) -> Option<Span> {
        self.functions
            .get(function as usize)
            .and_then(|function| function.provenance.span)
    }

    #[must_use]
    pub fn instruction_span(&self, function: u32, instruction: u32) -> Option<Span> {
        self.functions
            .get(function as usize)
            .and_then(|function| function.provenance.instruction_span(instruction))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BytecodeConst {
    Int(i64),
    Float(f64),
    Size(u64),
    Byte(u8),
    Bool(bool),
    None,
    String(String),
}
