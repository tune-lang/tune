use tune_bytecode::Opcode;
use tune_bytecode::function::{
    BytecodeOwnershipPlan, BytecodeStateRepr, BytecodeStructState, BytecodeVariant,
};
use tune_runtime::{
    state::{StateHandle, StateId},
    task::{Task, TaskJoinOutcome},
    value::{PropagationFrame, RuntimeVariant, Value},
};

use crate::{Vm, VmError, VmFault, VmLocation};

impl Vm {
    pub(crate) fn at<T>(
        &self,
        function: usize,
        instruction: usize,
        result: Result<T, VmError>,
    ) -> Result<T, VmFault> {
        result.map_err(|error| self.fault_at(function, instruction, error))
    }

    pub(crate) fn function_fault(&self, function: usize, error: VmError) -> VmFault {
        let Ok(function) = u32::try_from(function) else {
            return VmFault::new(error, None);
        };
        VmFault::new(
            error,
            Some(VmLocation {
                function,
                function_name: self.function_name(function),
                instruction: None,
                span: self.artifact.function_span(function),
            }),
        )
    }

    pub(crate) fn fault_at(&self, function: usize, instruction: usize, error: VmError) -> VmFault {
        let (Ok(function), Ok(instruction)) = (u32::try_from(function), u32::try_from(instruction))
        else {
            return VmFault::new(error, None);
        };
        VmFault::new(
            error,
            Some(VmLocation {
                function,
                function_name: self.function_name(function),
                instruction: Some(instruction),
                span: self.artifact.instruction_span(function, instruction),
            }),
        )
    }

    pub(crate) fn alloc_state(&self, state: BytecodeStructState) -> Result<StateHandle, VmError> {
        if !matches!(
            (state.repr, state.ownership),
            (
                BytecodeStateRepr::LocalHandle,
                BytecodeOwnershipPlan::NonAtomicRc
            ) | (
                BytecodeStateRepr::SharedHandle,
                BytecodeOwnershipPlan::SharedAtomic
            )
        ) {
            return Err(VmError::UnsupportedStructState);
        }
        let id = StateId(self.next_state_id.get());
        self.next_state_id
            .set(self.next_state_id.get().saturating_add(1));
        Ok(StateHandle {
            id,
            repr: crate::vm_state::runtime_state_repr(state.repr),
            ownership: crate::vm_state::runtime_ownership(state.ownership),
        })
    }

    pub(crate) fn push_ready_task(&self, value: Value) -> Value {
        let mut tasks = self.tasks.borrow_mut();
        let id = tune_runtime::TaskId(u64::try_from(tasks.len()).unwrap_or(u64::MAX));
        tasks.push(Task::ready(id, value));
        Value::Task(tune_runtime::TaskHandle(id))
    }

    pub(crate) fn join_task(&self, id: tune_runtime::TaskId) -> Option<TaskJoinOutcome> {
        self.tasks
            .borrow()
            .get(id.0 as usize)
            .cloned()
            .map(Task::join)
    }

    pub(crate) fn propagate_result(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        dst: u32,
        result: Value,
    ) -> Result<Option<Value>, VmFault> {
        match result {
            Value::Variant {
                variant: RuntimeVariant::ResultOk,
                mut fields,
                ..
            } if fields.len() == 1 => {
                self.at(
                    function,
                    instruction,
                    write_reg(registers, dst, fields.remove(0)),
                )?;
                Ok(None)
            }
            Value::Variant {
                variant: RuntimeVariant::ResultError,
                fields,
                mut propagation_frames,
            } => {
                if let Some(frame) = self.propagation_frame(function, instruction) {
                    propagation_frames.push(frame);
                }
                Ok(Some(Value::Variant {
                    variant: RuntimeVariant::ResultError,
                    fields,
                    propagation_frames,
                }))
            }
            _ => Err(self.fault_at(
                function,
                instruction,
                VmError::UnsupportedOpcode(Opcode::ResultPropagate),
            )),
        }
    }

    fn propagation_frame(&self, function: usize, instruction: usize) -> Option<PropagationFrame> {
        let function_index = u32::try_from(function).ok()?;
        let instruction_index = u32::try_from(instruction).ok()?;
        let function_name = self.function_name(function_index)?;
        Some(PropagationFrame {
            function: function_index,
            instruction: instruction_index,
            function_name,
            span: self
                .artifact
                .instruction_span(function_index, instruction_index),
        })
    }

    fn function_name(&self, function: u32) -> Option<String> {
        self.artifact
            .functions
            .get(function as usize)
            .map(|function| function.name.clone())
    }
}

pub(crate) fn read_reg(registers: &[Value], reg: u32) -> Result<Value, VmError> {
    registers
        .get(reg as usize)
        .cloned()
        .ok_or(VmError::RegisterOutOfBounds)
}

pub(crate) fn write_reg(registers: &mut [Value], reg: u32, value: Value) -> Result<(), VmError> {
    let slot = registers
        .get_mut(reg as usize)
        .ok_or(VmError::RegisterOutOfBounds)?;
    *slot = value;
    Ok(())
}

pub(crate) fn runtime_variant(variant: BytecodeVariant) -> RuntimeVariant {
    match variant {
        BytecodeVariant::ResultOk => RuntimeVariant::ResultOk,
        BytecodeVariant::ResultError => RuntimeVariant::ResultError,
        BytecodeVariant::Other { owner, index } => RuntimeVariant::Other { owner, index },
    }
}
