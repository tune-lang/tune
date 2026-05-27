pub mod artifact;
pub mod function;
pub mod lower;
pub mod opcode;

pub use lower::{BytecodeLowerError, lower_ir_function, lower_ir_functions};
pub use opcode::Opcode;
