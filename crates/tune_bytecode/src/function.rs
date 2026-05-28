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
pub struct BytecodeVariantSite {
    pub variant: BytecodeVariant,
    pub args: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct BytecodeStructSite {
    pub owner: u32,
    pub state: BytecodeStructState,
    pub fields: Vec<BytecodeStructField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BytecodeStructState {
    pub repr: BytecodeStateRepr,
    pub ownership: BytecodeOwnershipPlan,
}

impl BytecodeStructState {
    pub const LOCAL: Self = Self {
        repr: BytecodeStateRepr::LocalHandle,
        ownership: BytecodeOwnershipPlan::NonAtomicRc,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeStateRepr {
    Inline,
    LocalHandle,
    SharedHandle,
    HostResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeOwnershipPlan {
    Stack,
    DirectDrop,
    NonAtomicRc,
    Cow,
    SharedAtomic,
    HostRetained,
}

#[derive(Debug, Clone)]
pub struct BytecodeStructField {
    pub field: u32,
    pub value: u32,
}

#[derive(Debug, Clone)]
pub struct BytecodeMatchSite {
    pub arms: Vec<BytecodeMatchArm>,
}

#[derive(Debug, Clone)]
pub struct BytecodeMatchArm {
    pub variant: BytecodeVariant,
    pub target: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeVariant {
    ResultOk,
    ResultError,
    Other { owner: u32, index: u32 },
}

#[derive(Debug, Clone)]
pub struct BytecodeFunction {
    pub name: String,
    pub param_count: u32,
    pub register_count: u32,
    pub local_count: u32,
    pub call_sites: Vec<BytecodeCallSite>,
    pub struct_sites: Vec<BytecodeStructSite>,
    pub variant_sites: Vec<BytecodeVariantSite>,
    pub match_sites: Vec<BytecodeMatchSite>,
    pub instructions: Vec<Instruction>,
}
