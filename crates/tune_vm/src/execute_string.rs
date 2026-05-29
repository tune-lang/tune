use tune_bytecode::function::Instruction;
use tune_runtime::text;
use tune_runtime::value::Value;

use crate::execute_support::{read_reg_ref, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_string_build(
        &self,
        function: usize,
        instruction_index: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let bytecode_function = self.artifact.functions.get(function).ok_or_else(|| {
            self.fault_at(function, instruction_index, VmError::FunctionOutOfBounds)
        })?;
        let site = bytecode_function
            .string_sites
            .get(instruction.b as usize)
            .ok_or_else(|| {
                self.fault_at(function, instruction_index, VmError::CallSiteOutOfBounds)
            })?;
        let mut output = String::new();
        for part in &site.parts {
            let value = read_reg_ref(registers, *part)
                .map_err(|error| self.fault_at(function, instruction_index, error))?;
            append_string_part(&mut output, value);
        }
        self.at(
            function,
            instruction_index,
            write_reg(registers, instruction.a, Value::String(output)),
        )
    }

    pub(crate) fn execute_string_len(
        &self,
        function: usize,
        instruction_index: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let value = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function, instruction_index, error))?;
        let Value::String(value) = value else {
            return Err(self.fault_at(
                function,
                instruction_index,
                VmError::UnsupportedOpcode(instruction.opcode),
            ));
        };
        let len = u64::try_from(text::character_len(value))
            .map_err(|_| self.fault_at(function, instruction_index, VmError::NumericOverflow))?;
        self.at(
            function,
            instruction_index,
            write_reg(registers, instruction.a, Value::Size(len)),
        )
    }

    pub(crate) fn execute_string_get(
        &self,
        function: usize,
        instruction_index: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let value = read_reg_ref(registers, instruction.b)
            .map_err(|error| self.fault_at(function, instruction_index, error))?;
        let index = read_reg_ref(registers, instruction.c)
            .map_err(|error| self.fault_at(function, instruction_index, error))?;
        let (Value::String(value), Value::Size(index)) = (value, index) else {
            return Err(self.fault_at(
                function,
                instruction_index,
                VmError::UnsupportedOpcode(instruction.opcode),
            ));
        };
        let Some(value) = usize::try_from(*index)
            .ok()
            .and_then(|index| text::character_at(value, index))
        else {
            return Err(self.fault_at(
                function,
                instruction_index,
                VmError::SequenceIndexOutOfBounds,
            ));
        };
        self.at(
            function,
            instruction_index,
            write_reg(registers, instruction.a, Value::String(value)),
        )
    }
}

fn append_string_part(output: &mut String, value: &Value) {
    match value {
        Value::String(value) => output.push_str(value),
        Value::Int(value) => output.push_str(&value.to_string()),
        Value::Float(value) => output.push_str(&value.to_string()),
        Value::Size(value) => output.push_str(&value.to_string()),
        Value::Byte(value) => output.push_str(&value.to_string()),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Unit => output.push_str("()"),
        value => output.push_str(&format!("{value:?}")),
    }
}
