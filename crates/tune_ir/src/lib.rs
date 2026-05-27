pub mod ir;
pub mod lower;
mod lower_control;
mod lower_slots;

pub use ir::{
    BlockId, ConstId, FieldId, HostSymbolId, IrBlock, IrConst, IrFunction, IrOp, Reg, VariantArm,
};
pub use lower::{IrLowerError, lower_plan_function};
