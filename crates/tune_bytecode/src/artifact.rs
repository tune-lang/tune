use crate::function::BytecodeFunction;
use tune_diagnostics::Span;

#[derive(Debug, Clone)]
pub struct BytecodeArtifact {
    pub entry_function: Option<u32>,
    pub functions: Vec<BytecodeFunction>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeConst {
    Int(i64),
    Bool(bool),
}
