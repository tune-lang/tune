use tune_bytecode::{Opcode, function::Instruction};
use tune_runtime::value::Value;

use crate::execute_support::{read_reg_ref, write_reg};
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
                let left = read_reg_ref(registers, instruction.b)
                    .map_err(|error| self.fault_at(function_index, ip, error))?;
                let right = read_reg_ref(registers, instruction.c)
                    .map_err(|error| self.fault_at(function_index, ip, error))?;
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
                let left = read_reg_ref(registers, instruction.b)
                    .map_err(|error| self.fault_at(function_index, ip, error))?;
                let right = read_reg_ref(registers, instruction.c)
                    .map_err(|error| self.fault_at(function_index, ip, error))?;
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
                let (left, right) =
                    self.read_size_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_add(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_size(function_index, ip, registers, instruction, value)
            }
            Opcode::SubSizeChecked => {
                let (left, right) =
                    self.read_size_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_sub(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_size(function_index, ip, registers, instruction, value)
            }
            Opcode::MulSizeChecked => {
                let (left, right) =
                    self.read_size_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_mul(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.write_size(function_index, ip, registers, instruction, value)
            }
            Opcode::DivSize => {
                let (left, right) =
                    self.read_size_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_div(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, divide_size_error(right)))?;
                self.write_size(function_index, ip, registers, instruction, value)
            }
            Opcode::RemSize => {
                let (left, right) =
                    self.read_size_pair(function_index, ip, registers, instruction)?;
                let value = left
                    .checked_rem(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, divide_size_error(right)))?;
                self.write_size(function_index, ip, registers, instruction, value)
            }
            Opcode::AddByteWrap => {
                let (left, right) =
                    self.read_byte_pair(function_index, ip, registers, instruction)?;
                self.write_byte(
                    function_index,
                    ip,
                    registers,
                    instruction,
                    left.wrapping_add(right),
                )
            }
            Opcode::SubByteWrap => {
                let (left, right) =
                    self.read_byte_pair(function_index, ip, registers, instruction)?;
                self.write_byte(
                    function_index,
                    ip,
                    registers,
                    instruction,
                    left.wrapping_sub(right),
                )
            }
            Opcode::MulByteWrap => {
                let (left, right) =
                    self.read_byte_pair(function_index, ip, registers, instruction)?;
                self.write_byte(
                    function_index,
                    ip,
                    registers,
                    instruction,
                    left.wrapping_mul(right),
                )
            }
            Opcode::DivByte => {
                let (left, right) =
                    self.read_byte_pair(function_index, ip, registers, instruction)?;
                if right == 0 {
                    return Err(self.fault_at(function_index, ip, VmError::DivideByZero));
                }
                self.write_byte(function_index, ip, registers, instruction, left / right)
            }
            Opcode::RemByte => {
                let (left, right) =
                    self.read_byte_pair(function_index, ip, registers, instruction)?;
                if right == 0 {
                    return Err(self.fault_at(function_index, ip, VmError::DivideByZero));
                }
                self.write_byte(function_index, ip, registers, instruction, left % right)
            }
            Opcode::BitNotByte => {
                let value = read_reg_ref(registers, instruction.b)
                    .map_err(|error| self.fault_at(function_index, ip, error))?;
                let Value::Byte(value) = value else {
                    return Err(self.fault_at(
                        function_index,
                        ip,
                        VmError::UnsupportedOpcode(Opcode::BitNotByte),
                    ));
                };
                self.write_byte(function_index, ip, registers, instruction, !value)
            }
            Opcode::BitAndByte
            | Opcode::BitOrByte
            | Opcode::BitXorByte
            | Opcode::ShiftLeftByte
            | Opcode::ShiftRightByte => {
                let (left, right) =
                    self.read_byte_pair(function_index, ip, registers, instruction)?;
                let value = match instruction.opcode {
                    Opcode::BitAndByte => left & right,
                    Opcode::BitOrByte => left | right,
                    Opcode::BitXorByte => left ^ right,
                    Opcode::ShiftLeftByte => left.wrapping_shl(u32::from(right)),
                    Opcode::ShiftRightByte => left.wrapping_shr(u32::from(right)),
                    _ => unreachable!(),
                };
                self.write_byte(function_index, ip, registers, instruction, value)
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
        let left = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let right = read_reg_ref(registers, instruction.c)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let left = match left {
            Value::Int(left) => *left,
            _ => {
                return Err(self.fault_at(
                    function_index,
                    ip,
                    VmError::UnsupportedOpcode(instruction.opcode),
                ));
            }
        };
        let right = match right {
            Value::Int(right) => *right,
            _ => {
                return Err(self.fault_at(
                    function_index,
                    ip,
                    VmError::UnsupportedOpcode(instruction.opcode),
                ));
            }
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

    fn read_size_pair(
        &self,
        function_index: usize,
        ip: usize,
        registers: &[Value],
        instruction: &Instruction,
    ) -> Result<(u64, u64), VmFault> {
        let left = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let right = read_reg_ref(registers, instruction.c)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let left = match left {
            Value::Size(left) => *left,
            _ => {
                return Err(self.fault_at(
                    function_index,
                    ip,
                    VmError::UnsupportedOpcode(instruction.opcode),
                ));
            }
        };
        let right = match right {
            Value::Size(right) => *right,
            _ => {
                return Err(self.fault_at(
                    function_index,
                    ip,
                    VmError::UnsupportedOpcode(instruction.opcode),
                ));
            }
        };
        Ok((left, right))
    }

    fn write_size(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
        value: u64,
    ) -> Result<(), VmFault> {
        self.at(
            function_index,
            ip,
            write_reg(registers, instruction.a, Value::Size(value)),
        )
    }

    fn read_byte_pair(
        &self,
        function_index: usize,
        ip: usize,
        registers: &[Value],
        instruction: &Instruction,
    ) -> Result<(u8, u8), VmFault> {
        let left = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let right = read_reg_ref(registers, instruction.c)
            .map_err(|error| self.fault_at(function_index, ip, error))?;
        let left = match left {
            Value::Byte(left) => *left,
            _ => {
                return Err(self.fault_at(
                    function_index,
                    ip,
                    VmError::UnsupportedOpcode(instruction.opcode),
                ));
            }
        };
        let right = match right {
            Value::Byte(right) => *right,
            _ => {
                return Err(self.fault_at(
                    function_index,
                    ip,
                    VmError::UnsupportedOpcode(instruction.opcode),
                ));
            }
        };
        Ok((left, right))
    }

    fn write_byte(
        &self,
        function_index: usize,
        ip: usize,
        registers: &mut [Value],
        instruction: &Instruction,
        value: u8,
    ) -> Result<(), VmFault> {
        self.at(
            function_index,
            ip,
            write_reg(registers, instruction.a, Value::Byte(value)),
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

const fn divide_size_error(rhs: u64) -> VmError {
    if rhs == 0 {
        VmError::DivideByZero
    } else {
        VmError::NumericOverflow
    }
}
