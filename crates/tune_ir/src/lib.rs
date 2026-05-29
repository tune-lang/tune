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

pub use ir::{
    BlockId, ConstId, FieldId, HostSymbolId, IrBlock, IrByteBinary, IrCapture, IrCaptureMode,
    IrConst, IrFunction, IrIntComparison, IrOp, IrOwnershipPlan, IrStateRepr, IrStructLayout,
    IrStructState, Reg, StructField, VariantArm,
};
pub use lower::{IrLowerError, lower_plan_function};
