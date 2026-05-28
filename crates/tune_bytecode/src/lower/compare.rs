use tune_ir::IrIntComparison;

use crate::Opcode;

pub(super) fn lower_int_comparison(op: IrIntComparison) -> Opcode {
    match op {
        IrIntComparison::Equal => Opcode::EqualInt,
        IrIntComparison::NotEqual => Opcode::NotEqualInt,
        IrIntComparison::Less => Opcode::LessInt,
        IrIntComparison::LessEqual => Opcode::LessEqualInt,
        IrIntComparison::GreaterEqual => Opcode::GreaterEqualInt,
    }
}
