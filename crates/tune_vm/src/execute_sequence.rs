use tune_bytecode::Opcode;
use tune_bytecode::function::Instruction;
use tune_runtime::value::Value;

use crate::execute_support::{read_reg, read_reg_ref, write_reg};
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
            Opcode::SeqPush | Opcode::SeqPushExclusive | Opcode::SeqPushShared => {
                self.execute_sequence_push(function, instruction, registers, op)
            }
            Opcode::SeqGetChecked | Opcode::SeqGetUnchecked => {
                self.execute_sequence_get(function, instruction, registers, op)
            }
            Opcode::SeqSetChecked
            | Opcode::SeqSetUnchecked
            | Opcode::SeqSetCheckedExclusive
            | Opcode::SeqSetUncheckedExclusive
            | Opcode::SeqSetCheckedShared
            | Opcode::SeqSetUncheckedShared => {
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
        let value = self.at(function, instruction, read_reg(registers, op.b))?;
        let Value::Sequence(values) = registers
            .get_mut(op.a as usize)
            .ok_or_else(|| self.fault_at(function, instruction, VmError::RegisterOutOfBounds))?
        else {
            return Err(self.fault_at(
                function,
                instruction,
                VmError::UnsupportedOpcode(op.opcode),
            ));
        };
        values.push(value);
        Ok(())
    }

    fn execute_sequence_get(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let seq = self.at(function, instruction, read_reg_ref(registers, op.b))?;
        let index = self.at(function, instruction, read_reg_ref(registers, op.c))?;
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
        let index = self.at(function, instruction, read_reg(registers, op.b))?;
        let value = self.at(function, instruction, read_reg(registers, op.c))?;
        let Value::Sequence(values) = registers
            .get_mut(op.a as usize)
            .ok_or_else(|| self.fault_at(function, instruction, VmError::RegisterOutOfBounds))?
        else {
            return Err(self.fault_at(
                function,
                instruction,
                VmError::UnsupportedOpcode(op.opcode),
            ));
        };
        let index = sequence_index(&index).ok_or_else(|| {
            self.fault_at(function, instruction, VmError::UnsupportedOpcode(op.opcode))
        })?;
        let Some(slot) = values.get_mut(index) else {
            return Err(self.fault_at(function, instruction, VmError::RegisterOutOfBounds));
        };
        *slot = value;
        Ok(())
    }
}

fn sequence_get(opcode: Opcode, seq: &Value, index: &Value) -> Option<Value> {
    let Value::Sequence(values) = seq else {
        return None;
    };
    let index = sequence_index(index)?;
    match opcode {
        Opcode::SeqGetChecked | Opcode::SeqGetUnchecked => values.get(index).cloned(),
        _ => None,
    }
}

fn sequence_index(index: &Value) -> Option<usize> {
    match index {
        Value::Int(index) => usize::try_from(*index).ok(),
        Value::Size(index) => usize::try_from(*index).ok(),
        _ => None,
    }
}
