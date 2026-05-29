use tune_bytecode::function::Instruction;
use tune_runtime::value::Value;

use crate::execute_support::{read_reg, write_reg};
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
            let value = self.at(function, instruction_index, read_reg(registers, *part))?;
            append_string_part(&mut output, &value);
        }
        self.at(
            function,
            instruction_index,
            write_reg(registers, instruction.a, Value::String(output)),
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
