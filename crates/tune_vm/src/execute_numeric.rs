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
                let left = self.at(function_index, ip, read_reg(registers, instruction.b))?;
                let right = self.at(function_index, ip, read_reg(registers, instruction.c))?;
                let (Value::Int(left), Value::Int(right)) = (left, right) else {
                    return Err(self.fault_at(
                        function_index,
                        ip,
                        VmError::UnsupportedOpcode(Opcode::AddInt),
                    ));
                };
                let value = left
                    .checked_add(right)
                    .ok_or_else(|| self.fault_at(function_index, ip, VmError::NumericOverflow))?;
                self.at(
                    function_index,
                    ip,
                    write_reg(registers, instruction.a, Value::Int(value)),
                )
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
}
