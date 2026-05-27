use crate::Opcode;

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

#[derive(Debug, Clone)]
pub struct BytecodeCallSite {
    pub function: u32,
    pub args: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct BytecodeFunction {
    pub name: String,
    pub register_count: u32,
    pub local_count: u32,
    pub call_sites: Vec<BytecodeCallSite>,
    pub instructions: Vec<Instruction>,
}
