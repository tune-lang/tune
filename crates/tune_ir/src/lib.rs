pub mod ir;
pub mod lower;
mod lower_aggregate;
mod lower_binary;
mod lower_bindings;
mod lower_bool;
mod lower_byte;
mod lower_calls;
mod lower_control;
mod lower_for;
mod lower_sequence;
mod lower_slots;
mod lower_state;
mod lower_tasks;
mod lower_unary;
mod ownership;
mod provenance;

pub use ir::{
    BlockId, ConstId, FieldId, IrBlock, IrByteBinary, IrCapture, IrCaptureMode, IrConst,
    IrFunction, IrGenericStrategy, IrIntComparison, IrOp, IrOwnershipPlan, IrStateRepr,
    IrStructLayout, IrStructState, Reg, StructField, VariantArm,
};
pub use lower::{IrLowerError, lower_plan_function};
pub use ownership::{IrLocalAccess, IrLocalStore, IrTransfer};
pub use tune_host::HostSymbolId;
pub use tune_resolve::LocalId;
