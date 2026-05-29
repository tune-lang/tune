use tune_bytecode::{Opcode, function::Instruction};
use tune_runtime::value::Value;

use crate::execute_support::{read_reg, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_add(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        match instruction.opcode {
            Opcode::AddInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_add(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_int(function_index, ip, registers, instruction, value)
            }
            Opcode::SubInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_sub(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_int(function_index, ip, registers, instruction, value)
            }
            Opcode::MulInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_mul(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_int(function_index, ip, registers, instruction, value)
            }
            Opcode::DivInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_div(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, divide_error(right)))?;
                self.write_int(function_index, ip, registers, instruction, value)
            }
            Opcode::RemInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_rem(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, divide_error(right)))?;
                self.write_int(function_index, ip, registers, instruction, value)
            }
            Opcode::BitAndInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                self.write_int(function_index, ip, registers, instruction, left & right)
            }
            Opcode::BitOrInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                self.write_int(function_index, ip, registers, instruction, left | right)
            }
            Opcode::BitXorInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                self.write_int(function_index, ip, registers, instruction, left ^ right)
            }
            Opcode::ShiftLeftInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                let shift = u32::try_from(right)
                    .ok()
                    .filter(|shift| *shift < i64::BITS)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                let value = left
                    .checked_shl(shift)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_int(function_index, ip, registers, instruction, value)
            }
            Opcode::ShiftRightInt => {
                let (left, right) =
                    self.read_int_pair(function_index, ip, registers, instruction)?;
                let shift = u32::try_from(right)
                    .ok()
                    .filter(|shift| *shift < i64::BITS)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                let value = left
                    .checked_shr(shift)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_int(function_index, ip, registers, instruction, value)
            }
            Opcode::AddFloat => {
                let left = self.at(function_index, ip, read_reg(registers, instruction.b))?;
                let right = self.at(function_index, ip, read_reg(registers, instruction.c))?;
                let (Value::Float(left), Value::Float(right)) = (left, right) else {
                    return Err(self.fault_at(
                        function_index,
                        ip,
                        VmError::UnsupportedOpcode(Opcode::AddFloat),
                    ));
                };
                self.at(
                    function_index,
                    ip,
                    write_reg(registers, instruction.a, Value::Float(left + right)),
                )
            }
            Opcode::SubFloat | Opcode::MulFloat | Opcode::DivFloat => {
                let left = self.at(function_index, ip, read_reg(registers, instruction.b))?;
                let right = self.at(function_index, ip, read_reg(registers, instruction.c))?;
                let (Value::Float(left), Value::Float(right)) = (left, right) else {
                    return Err(self.fault_at(
                        function_index,
                        ip,
                        VmError::UnsupportedOpcode(instruction.opcode),
                    ));
                };
                let value = match instruction.opcode {
                    Opcode::SubFloat => left - right,
                    Opcode::MulFloat => left * right,
                    Opcode::DivFloat => left / right,
                    _ => unreachable!(),
                };
                if !value.is_finite() {
                    return Err(self.fault_at(function_index, ip, VmError::NumericOverflow));
                }
                self.at(
                    function_index,
                    ip,
                    write_reg(registers, instruction.a, Value::Float(value)),
                )
            }
            Opcode::AddSizeChecked => {
                let left = self.at(function_index, ip, read_reg(registers, instruction.b))?;
                let right = self.at(function_index, ip, read_reg(registers, instruction.c))?;
                let (Value::Size(left), Value::Size(right)) = (left, right) else {
                    return Err(self.fault_at(
                        function_index,
                        ip,
                        VmError::UnsupportedOpcode(Opcode::AddSizeChecked),
                    ));
                };
                let value = left
                    .checked_add(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.at(
                    function_index,
                    ip,
                    write_reg(registers, instruction.a, Value::Size(value)),
                )
            }
            Opcode::AddByteWrap => {
                let left = self.at(function_index, ip, read_reg(registers, instruction.b))?;
                let right = self.at(function_index, ip, read_reg(registers, instruction.c))?;
                let (Value::Byte(left), Value::Byte(right)) = (left, right) else {
                    return Err(self.fault_at(
                        function_index,
                        ip,
                        VmError::UnsupportedOpcode(Opcode::AddByteWrap),
                    ));
                };
                self.at(
                    function_index,
                    ip,
                    write_reg(
                        registers,
                        instruction.a,
                        Value::Byte(left.wrapping_add(right)),
                    ),
                )
            }
            other => Err(self.fault_at(function_index, ip, VmError::UnsupportedOpcode(other))),
        }
    }

    fn read_int_pair(
        &self,
        function_index: usize,
        ip: usize,
        registers: &[Value],
        instruction: &Instruction,
    ) -> Result<(i64, i64), VmFault> {
        let left = self.at(function_index, ip, read_reg(registers, instruction.b))?;
        let right = self.at(function_index, ip, read_reg(registers, instruction.c))?;
        let (Value::Int(left), Value::Int(right)) = (left, right) else {
            return Err(self.fault_at(
                function_index,
                ip,
                VmError::UnsupportedOpcode(instruction.opcode),
            ));
        };
        Ok((left, right))
    }

    fn write_int(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
        value: i64,
    ) -> Result<(), VmFault> {
        self.at(
            function_index,
            ip,
            write_reg(registers, instruction.a, Value::Int(value)),
        )
    }
}

const fn divide_error(rhs: i64) -> VmError {
    if rhs == 0 {
        VmError::DivideByZero
    } else {
        VmError::NumericOverflow
    }
}
