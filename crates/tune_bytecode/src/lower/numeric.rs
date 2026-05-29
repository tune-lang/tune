use tune_ir::IrOp;

use crate::Opcode;
use crate::lower::compare::{lower_float_comparison, lower_int_comparison};
use crate::lower::{BytecodeLowerError, FunctionLowerer};

impl FunctionLowerer<'_> {
    pub(super) fn lower_numeric_op(&mut self, op: &IrOp) -> Result<(), BytecodeLowerError> {
        match op {
            IrOp::AddInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::AddInt, dst.0, a.0, b.0)
            }
            IrOp::SubInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::SubInt, dst.0, a.0, b.0)
            }
            IrOp::MulInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::MulInt, dst.0, a.0, b.0)
            }
            IrOp::DivInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::DivInt, dst.0, a.0, b.0)
            }
            IrOp::RemInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::RemInt, dst.0, a.0, b.0)
            }
            IrOp::BitAndInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::BitAndInt, dst.0, a.0, b.0);
            }
            IrOp::BitOrInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::BitOrInt, dst.0, a.0, b.0);
            }
            IrOp::BitXorInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::BitXorInt, dst.0, a.0, b.0);
            }
            IrOp::ShiftLeftInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::ShiftLeftInt, dst.0, a.0, b.0);
            }
            IrOp::ShiftRightInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::ShiftRightInt, dst.0, a.0, b.0);
            }
            IrOp::AddFloat { dst, a, b, .. } => {
                self.push_instruction(Opcode::AddFloat, dst.0, a.0, b.0);
            }
            IrOp::SubFloat { dst, a, b, .. } => {
                self.push_instruction(Opcode::SubFloat, dst.0, a.0, b.0);
            }
            IrOp::MulFloat { dst, a, b, .. } => {
                self.push_instruction(Opcode::MulFloat, dst.0, a.0, b.0);
            }
            IrOp::DivFloat { dst, a, b, .. } => {
                self.push_instruction(Opcode::DivFloat, dst.0, a.0, b.0);
            }
            IrOp::AddSizeChecked { dst, a, b, .. } => {
                self.push_instruction(Opcode::AddSizeChecked, dst.0, a.0, b.0);
            }
            IrOp::AddByteWrap { dst, a, b } => {
                self.push_instruction(Opcode::AddByteWrap, dst.0, a.0, b.0);
            }
            IrOp::NegInt { dst, value, .. } => {
                self.push_instruction(Opcode::NegInt, dst.0, value.0, 0);
            }
            IrOp::NotBool { dst, value, .. } => {
                self.push_instruction(Opcode::NotBool, dst.0, value.0, 0);
            }
            IrOp::BitNotInt { dst, value, .. } => {
                self.push_instruction(Opcode::BitNotInt, dst.0, value.0, 0);
            }
            IrOp::NoneCheck {
                dst, value, is_not, ..
            } => self.push_instruction(Opcode::NoneCheck, dst.0, value.0, u32::from(*is_not)),
            IrOp::GreaterInt { dst, a, b, .. } => {
                self.push_instruction(Opcode::GreaterInt, dst.0, a.0, b.0);
            }
            IrOp::GreaterFloat { dst, a, b, .. } => {
                self.push_instruction(Opcode::GreaterFloat, dst.0, a.0, b.0);
            }
            IrOp::CompareInt { dst, a, b, op, .. } => {
                self.push_instruction(lower_int_comparison(*op), dst.0, a.0, b.0);
            }
            IrOp::CompareFloat { dst, a, b, op, .. } => {
                self.push_instruction(lower_float_comparison(*op), dst.0, a.0, b.0);
            }
            _ => return Err(BytecodeLowerError::UnsupportedIr("numeric ir op")),
        }
        Ok(())
    }
}
