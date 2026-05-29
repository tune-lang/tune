use tune_bytecode::{BytecodeValidationError, Opcode};
use tune_diagnostics::Span;
use tune_runtime::TunePanic;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmLocation {
    pub function: u32,
    pub function_name: Option<String>,
    pub instruction: Option<u32>,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmFault {
    pub error: VmError,
    pub location: Option<VmLocation>,
}

impl VmFault {
    #[must_use]
    pub const fn new(error: VmError, location: Option<VmLocation>) -> Self {
        Self { error, location }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    MissingEntry,
    RegisterOutOfBounds,
    ConstantOutOfBounds,
    FunctionOutOfBounds,
    CallSiteOutOfBounds,
    StructSiteOutOfBounds,
    ForSiteOutOfBounds,
    PanicSiteOutOfBounds,
    ArityMismatch,
    NumericOverflow,
    DivideByZero,
    SequenceIndexOutOfBounds,
    TaskUnsafeCapture { resource_type: String },
    UnsupportedStructState,
    Panic(TunePanic),
    InvalidBytecode(BytecodeValidationError),
    UnsupportedOpcode(Opcode),
}
