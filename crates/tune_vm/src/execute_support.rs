use tune_bytecode::Opcode;
use tune_bytecode::function::{
    BytecodeOwnershipPlan, BytecodeStateRepr, BytecodeStructState, BytecodeVariant, Instruction,
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

    pub(crate) fn execute_int_comparison(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let left = self.at(function, instruction, read_reg(registers, op.b))?;
        let right = self.at(function, instruction, read_reg(registers, op.c))?;
        let (Value::Int(left), Value::Int(right)) = (left, right) else {
            return Err(self.fault_at(
                function,
                instruction,
                VmError::UnsupportedOpcode(op.opcode),
            ));
        };
        let result = self.at(function, instruction, compare_int(op.opcode, left, right))?;
        self.at(
            function,
            instruction,
            write_reg(registers, op.a, Value::Bool(result)),
        )
    }

    pub(crate) fn execute_unary(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let value = self.at(function, instruction, read_reg(registers, op.b))?;
        let result = match (op.opcode, value) {
            (Opcode::NegInt, Value::Int(value)) => value.checked_neg().map(Value::Int),
            (Opcode::NotBool, Value::Bool(value)) => Some(Value::Bool(!value)),
            _ => None,
        }
        .ok_or_else(|| {
            self.fault_at(function, instruction, VmError::UnsupportedOpcode(op.opcode))
        })?;
        self.at(function, instruction, write_reg(registers, op.a, result))
    }

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
            Opcode::SeqPush => {
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
            _ => Err(self.fault_at(function, instruction, VmError::UnsupportedOpcode(op.opcode))),
        }
    }

    pub(crate) fn execute_finite_for_init(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let iterable = self.at(function, instruction, read_reg(registers, op.b))?;
        let Value::Sequence(values) = iterable else {
            return Err(self.fault_at(
                function,
                instruction,
                VmError::UnsupportedOpcode(Opcode::FiniteForInit),
            ));
        };
        let len = i64::try_from(values.len()).map_err(|_| {
            self.fault_at(
                function,
                instruction,
                VmError::UnsupportedOpcode(Opcode::FiniteForInit),
            )
        })?;
        self.at(
            function,
            instruction,
            write_reg(registers, op.a, Value::Int(0)),
        )?;
        self.at(
            function,
            instruction,
            write_reg(registers, op.c, Value::Int(len)),
        )
    }

    pub(crate) fn execute_finite_for_next(
        &self,
        function_index: usize,
        instruction_index: usize,
        function: &tune_bytecode::function::BytecodeFunction,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<usize, VmFault> {
        let site = function.for_sites.get(op.b as usize).ok_or_else(|| {
            self.fault_at(
                function_index,
                instruction_index,
                VmError::ForSiteOutOfBounds,
            )
        })?;
        let iterator = self.at(function_index, instruction_index, read_reg(registers, op.a))?;
        let Value::Int(iterator) = iterator else {
            return Err(self.fault_at(
                function_index,
                instruction_index,
                VmError::UnsupportedOpcode(Opcode::FiniteForNext),
            ));
        };
        let len = self.at(
            function_index,
            instruction_index,
            read_reg(registers, site.len),
        )?;
        let Value::Int(len) = len else {
            return Err(self.fault_at(
                function_index,
                instruction_index,
                VmError::UnsupportedOpcode(Opcode::FiniteForNext),
            ));
        };
        if iterator >= len {
            return Ok(site.done as usize);
        }
        let iterable = self.at(
            function_index,
            instruction_index,
            read_reg(registers, site.iterable),
        )?;
        let Value::Sequence(values) = iterable else {
            return Err(self.fault_at(
                function_index,
                instruction_index,
                VmError::UnsupportedOpcode(Opcode::FiniteForNext),
            ));
        };
        let index = usize::try_from(iterator).map_err(|_| {
            self.fault_at(
                function_index,
                instruction_index,
                VmError::UnsupportedOpcode(Opcode::FiniteForNext),
            )
        })?;
        let item = values.get(index).cloned().ok_or_else(|| {
            self.fault_at(
                function_index,
                instruction_index,
                VmError::UnsupportedOpcode(Opcode::FiniteForNext),
            )
        })?;
        self.at(
            function_index,
            instruction_index,
            write_reg(registers, site.index, Value::Int(iterator)),
        )?;
        self.at(
            function_index,
            instruction_index,
            write_reg(registers, site.item, item),
        )?;
        self.at(
            function_index,
            instruction_index,
            write_reg(registers, op.a, Value::Int(iterator + 1)),
        )?;
        Ok(site.body as usize)
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

pub(crate) fn compare_int(opcode: Opcode, left: i64, right: i64) -> Result<bool, VmError> {
    match opcode {
        Opcode::GreaterInt => Ok(left > right),
        Opcode::EqualInt => Ok(left == right),
        Opcode::NotEqualInt => Ok(left != right),
        Opcode::LessInt => Ok(left < right),
        Opcode::LessEqualInt => Ok(left <= right),
        Opcode::GreaterEqualInt => Ok(left >= right),
        _ => Err(VmError::UnsupportedOpcode(opcode)),
    }
}
