pub mod ir;
pub mod lower;
mod lower_calls;
mod lower_control;
mod lower_slots;
mod lower_state;
mod lower_tasks;

pub use ir::{
    BlockId, ConstId, FieldId, HostSymbolId, IrBlock, IrConst, IrFunction, IrOp, IrOwnershipPlan,
    IrStateRepr, IrStructState, Reg, StructField, VariantArm,
};
pub use lower::{IrLowerError, lower_plan_function};
