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
    HostSymbolOutOfBounds,
    StructSiteOutOfBounds,
    ForSiteOutOfBounds,
    PanicSiteOutOfBounds,
    ArityMismatch,
    NumericOverflow,
    DivideByZero,
    SequenceIndexOutOfBounds,
    TaskUnsafeCapture { resource_type: String },
    TaskUnsafeHostCall { symbol: u32 },
    UnknownHostResourceType { resource_type: String },
    UnknownHostValueType { type_name: String },
    MissingHostValueField { type_name: String, field: String },
    MissingHostAuthority { authority: String },
    MissingHostExecutor { symbol: u32 },
    HostCallFailed { message: String },
    RecursiveStructState,
    UnsupportedStructState,
    Panic(TunePanic),
    InvalidBytecode(BytecodeValidationError),
    UnsupportedOpcode(Opcode),
}
