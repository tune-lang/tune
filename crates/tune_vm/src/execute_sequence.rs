use tune_bytecode::Opcode;
use tune_bytecode::function::Instruction;
use tune_runtime::value::Value;

use crate::execute_support::{read_reg, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_sequence(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        match op.opcode {
            Opcode::SeqBuild => self.at(
                function,
                instruction,
                write_reg(registers, op.a, Value::Sequence(Vec::new())),
            ),
            Opcode::SeqPush => self.execute_sequence_push(function, instruction, registers, op),
            Opcode::SeqGetChecked | Opcode::SeqGetUnchecked => {
                self.execute_sequence_get(function, instruction, registers, op)
            }
            Opcode::SeqSetChecked | Opcode::SeqSetUnchecked => {
                self.execute_sequence_set(function, instruction, registers, op)
            }
            _ => Err(self.fault_at(function, instruction, VmError::UnsupportedOpcode(op.opcode))),
        }
    }

    fn execute_sequence_push(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let seq = self.at(function, instruction, read_reg(registers, op.a))?;
        let value = self.at(function, instruction, read_reg(registers, op.b))?;
        let Value::Sequence(mut values) = seq else {
            return Err(self.fault_at(
                function,
                instruction,
                VmError::UnsupportedOpcode(Opcode::SeqPush),
            ));
        };
        values.push(value);
        self.at(
            function,
            instruction,
            write_reg(registers, op.a, Value::Sequence(values)),
        )
    }

    fn execute_sequence_get(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let seq = self.at(function, instruction, read_reg(registers, op.b))?;
        let index = self.at(function, instruction, read_reg(registers, op.c))?;
        let value = sequence_get(op.opcode, seq, index).ok_or_else(|| {
            self.fault_at(function, instruction, VmError::UnsupportedOpcode(op.opcode))
        })?;
        self.at(function, instruction, write_reg(registers, op.a, value))
    }

    fn execute_sequence_set(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let seq = self.at(function, instruction, read_reg(registers, op.a))?;
        let index = self.at(function, instruction, read_reg(registers, op.b))?;
        let value = self.at(function, instruction, read_reg(registers, op.c))?;
        let seq = sequence_set(op.opcode, seq, index, value).ok_or_else(|| {
            self.fault_at(function, instruction, VmError::UnsupportedOpcode(op.opcode))
        })?;
        self.at(function, instruction, write_reg(registers, op.a, seq))
    }
}

fn sequence_get(opcode: Opcode, seq: Value, index: Value) -> Option<Value> {
    let Value::Sequence(values) = seq else {
        return None;
    };
    let index = sequence_index(index)?;
    match opcode {
        Opcode::SeqGetChecked | Opcode::SeqGetUnchecked => values.get(index).cloned(),
        _ => None,
    }
}

fn sequence_set(opcode: Opcode, seq: Value, index: Value, value: Value) -> Option<Value> {
    let Value::Sequence(mut values) = seq else {
        return None;
    };
    let index = sequence_index(index)?;
    match opcode {
        Opcode::SeqSetChecked | Opcode::SeqSetUnchecked => {
            let slot = values.get_mut(index)?;
            *slot = value;
            Some(Value::Sequence(values))
        }
        _ => None,
    }
}

fn sequence_index(index: Value) -> Option<usize> {
    match index {
        Value::Int(index) => usize::try_from(index).ok(),
        Value::Size(index) => usize::try_from(index).ok(),
        _ => None,
    }
}
