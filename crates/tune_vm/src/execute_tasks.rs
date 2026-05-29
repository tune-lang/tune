use tune_bytecode::function::{BytecodeCaptureMode, Instruction};
use tune_runtime::{task::TaskExecutionMode, value::Value};

use crate::execute_support::{read_reg, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_spawn_task(
        &self,
        function: usize,
        instruction_index: usize,
        registers: &mut [Value],
        instruction: &Instruction,
    ) -> Result<(), VmFault> {
        let function_artifact = self
            .artifact
            .functions
            .get(function)
            .ok_or_else(|| VmFault::new(VmError::FunctionOutOfBounds, None))?;
        let site = function_artifact
            .task_sites
            .get(instruction.b as usize)
            .ok_or_else(|| {
                self.fault_at(function, instruction_index, VmError::CallSiteOutOfBounds)
            })?;
        let args = site
            .captures
            .iter()
            .map(|capture| {
                self.at(
                    function,
                    instruction_index,
                    read_reg(registers, capture.register),
                )
                .and_then(|value| match capture.mode {
                    BytecodeCaptureMode::Reference => Ok(value),
                    BytecodeCaptureMode::PrivateSnapshot => {
                        self.at(function, instruction_index, self.capture_snapshot(&value))
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(resource_type) = self.validate_task_args(&args) {
            return Err(self.fault_at(
                function,
                instruction_index,
                VmError::TaskUnsafeCapture { resource_type },
            ));
        }
        let task = match self.task_execution {
            TaskExecutionMode::Immediate => {
                let value = self.execute_task_function(site.function as usize, args)?;
                self.push_ready_task(value)
            }
            TaskExecutionMode::Parallel => {
                let vm = self.task_vm();
                let task_function = site.function as usize;
                let handle =
                    std::thread::spawn(move || vm.execute_task_function(task_function, args));
                self.push_running_task(handle)
            }
            TaskExecutionMode::DeferredUntilJoin => self.at(
                function,
                instruction_index,
                self.push_deferred_task(site.function, args),
            )?,
        };
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
            Value::Task(handle) => {
                if let Some(value) = self.ready_task_value(handle.0) {
                    return self.at(
                        function,
                        instruction_index,
                        write_reg(registers, instruction.a, value),
                    );
                }
                if let Some(join) = self.take_running_task(handle.0) {
                    let value = join.join().map_err(|_| {
                        self.fault_at(
                            function,
                            instruction_index,
                            VmError::HostCallFailed {
                                message: "parallel task panicked in host thread".into(),
                            },
                        )
                    })??;
                    self.finish_task(handle.0, value.clone());
                    return self.at(
                        function,
                        instruction_index,
                        write_reg(registers, instruction.a, value),
                    );
                }
                if let Some((task_function, task_locals)) = self.take_pending_task(handle.0) {
                    let value = self.execute_task_function(task_function as usize, task_locals)?;
                    self.finish_task(handle.0, value.clone());
                    return self.at(
                        function,
                        instruction_index,
                        write_reg(registers, instruction.a, value),
                    );
                }
                Err(self.fault_at(function, instruction_index, VmError::RegisterOutOfBounds))
            }
            _ => Err(self.fault_at(
                function,
                instruction_index,
                VmError::UnsupportedOpcode(tune_bytecode::Opcode::TaskJoin),
            )),
        }
    }
}
