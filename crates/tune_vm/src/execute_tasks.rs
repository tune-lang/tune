use tune_bytecode::function::Instruction;
use tune_runtime::{task::TaskJoinOutcome, value::Value};

use crate::execute_support::{read_reg, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_spawn_task(
        &self,
        function: usize,
        instruction_index: usize,
        locals: &[Value],
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let task = self.at(
            function,
            instruction_index,
            self.push_deferred_task(instruction.b, locals),
        )?;
        self.at(
            function,
            instruction_index,
            write_reg(registers, instruction.a, task),
        )
    }

    pub(crate) fn execute_task_join(
        &self,
        function: usize,
        instruction_index: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        match self.at(
            function,
            instruction_index,
            read_reg(registers, instruction.b),
        )? {
            Value::Task(handle) => match self.join_task(handle.0) {
                Some(TaskJoinOutcome::Ready(value)) => self.at(
                    function,
                    instruction_index,
                    write_reg(registers, instruction.a, value),
                ),
                Some(TaskJoinOutcome::Pending(id)) => {
                    let Some((task_function, task_locals)) = self.take_pending_task(id) else {
                        return Err(self.fault_at(
                            function,
                            instruction_index,
                            VmError::RegisterOutOfBounds,
                        ));
                    };
                    let value = self.execute_task_function(task_function as usize, task_locals)?;
                    self.finish_task(id, value.clone());
                    self.at(
                        function,
                        instruction_index,
                        write_reg(registers, instruction.a, value),
                    )
                }
                Some(TaskJoinOutcome::UnrecoverablePanic(_)) => Err(self.fault_at(
                    function,
                    instruction_index,
                    VmError::UnsupportedOpcode(tune_bytecode::Opcode::TaskJoin),
                )),
                None => {
                    Err(self.fault_at(function, instruction_index, VmError::RegisterOutOfBounds))
                }
            },
            _ => Err(self.fault_at(
                function,
                instruction_index,
                VmError::UnsupportedOpcode(tune_bytecode::Opcode::TaskJoin),
            )),
        }
    }
}
