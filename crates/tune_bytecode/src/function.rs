use crate::Opcode;
use crate::provenance::BytecodeFunctionProvenance;
use tune_shape::Shape;

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
    pub type_args: Vec<Shape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeFrameLayout {
    pub params: Vec<Shape>,
    pub locals: Vec<Shape>,
    pub registers: Vec<Shape>,
}

impl BytecodeFrameLayout {
    #[must_use]
    pub fn unknown(param_count: u32, register_count: u32, local_count: u32) -> Self {
        Self {
            params: vec![Shape::Hole; param_count as usize],
            locals: vec![Shape::Hole; local_count as usize],
            registers: vec![Shape::Hole; register_count as usize],
        }
    }
}

#[derive(Debug, Clone)]
pub struct BytecodeBoundCallSite {
    pub args: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct BytecodeCallableSite {
    pub function: u32,
    pub captures: Vec<BytecodeCapture>,
}

#[derive(Debug, Clone)]
pub struct BytecodeTaskSite {
    pub function: u32,
    pub captures: Vec<BytecodeCapture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BytecodeCapture {
    pub register: u32,
    pub mode: BytecodeCaptureMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytecodeCaptureMode {
    Reference,
    PrivateSnapshot,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeStructLayout {
    pub owner: u32,
    pub fields: Vec<u32>,
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
pub struct BytecodeFieldSite {
    pub owner: u32,
    pub field: u32,
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

#[derive(Debug, Clone)]
pub struct BytecodeForSite {
    pub iterable: u32,
    pub len: u32,
    pub index: u32,
    pub item: u32,
    pub body: u32,
    pub done: u32,
}

#[derive(Debug, Clone)]
pub struct BytecodePanicSite {
    pub args: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct BytecodeTupleSite {
    pub items: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct BytecodeStringSite {
    pub parts: Vec<u32>,
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
    pub provenance: BytecodeFunctionProvenance,
    pub param_count: u32,
    pub register_count: u32,
    pub local_count: u32,
    pub frame: BytecodeFrameLayout,
    pub call_sites: Vec<BytecodeCallSite>,
    pub bound_call_sites: Vec<BytecodeBoundCallSite>,
    pub callable_sites: Vec<BytecodeCallableSite>,
    pub task_sites: Vec<BytecodeTaskSite>,
    pub struct_sites: Vec<BytecodeStructSite>,
    pub field_sites: Vec<BytecodeFieldSite>,
    pub variant_sites: Vec<BytecodeVariantSite>,
    pub match_sites: Vec<BytecodeMatchSite>,
    pub for_sites: Vec<BytecodeForSite>,
    pub panic_sites: Vec<BytecodePanicSite>,
    pub tuple_sites: Vec<BytecodeTupleSite>,
    pub string_sites: Vec<BytecodeStringSite>,
    pub instructions: Vec<Instruction>,
}
