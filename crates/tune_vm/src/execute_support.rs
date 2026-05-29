use tune_bytecode::Opcode;
use tune_bytecode::function::{
    BytecodeOwnershipPlan, BytecodeStateRepr, BytecodeStructState, BytecodeVariant, Instruction,
};
use tune_runtime::{
    TunePanic,
    state::{StateHandle, StateId},
    task::{TaskId, TaskJoinOutcome},
    value::{PropagationFrame, RuntimeVariant, TaskHandle, Value},
};

use crate::execute_range::{range_item, range_len, value_range};
use crate::{Vm, VmError, VmFault, VmLocation, vm::VmTask};

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

    pub(crate) fn push_deferred_task(&self, function: u32, locals: &[Value]) -> Value {
        let mut tasks = self.tasks.borrow_mut();
        let id = TaskId(u64::try_from(tasks.len()).unwrap_or(u64::MAX));
        tasks.push(VmTask::Pending {
            id,
            function,
            locals: locals.to_vec(),
        });
        Value::Task(TaskHandle(id))
    }

    pub(crate) fn capture_snapshot(&self, value: &Value) -> Result<Value, VmError> {
        match value {
            Value::Struct { owner, fields } => {
                let state = self.alloc_state(BytecodeStructState {
                    repr: tune_bytecode::function::BytecodeStateRepr::LocalHandle,
                    ownership: tune_bytecode::function::BytecodeOwnershipPlan::NonAtomicRc,
                })?;
                Ok(Value::Struct {
                    owner: *owner,
                    fields: fields.snapshot_with_state(state),
                })
            }
            Value::Sequence(values) => values
                .iter()
                .map(|value| self.capture_snapshot(value))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Sequence),
            Value::Tuple(values) => values
                .iter()
                .map(|value| self.capture_snapshot(value))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Tuple),
            Value::Variant {
                variant,
                fields,
                propagation_frames,
            } => Ok(Value::Variant {
                variant: *variant,
                fields: fields
                    .iter()
                    .map(|value| self.capture_snapshot(value))
                    .collect::<Result<Vec<_>, _>>()?,
                propagation_frames: propagation_frames.clone(),
            }),
            value => Ok(value.clone()),
        }
    }

    pub(crate) fn join_task(&self, id: tune_runtime::TaskId) -> Option<TaskJoinOutcome> {
        let task = self.tasks.borrow().get(id.0 as usize).cloned()?;
        Some(task.join())
    }

    pub(crate) fn take_pending_task(&self, id: TaskId) -> Option<(u32, Vec<Value>)> {
        let task = self.tasks.borrow().get(id.0 as usize).cloned()?;
        match task {
            VmTask::Pending {
                function, locals, ..
            } => Some((function, locals)),
            VmTask::Ready { .. } => None,
        }
    }

    pub(crate) fn finish_task(&self, id: TaskId, value: Value) {
        if let Some(task) = self.tasks.borrow_mut().get_mut(id.0 as usize) {
            *task = VmTask::Ready { value };
        }
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
            (Opcode::BitNotInt, Value::Int(value)) => Some(Value::Int(!value)),
            (Opcode::NoneCheck, value) => {
                Some(Value::Bool(matches!(value, Value::None) != (op.c != 0)))
            }
            _ => None,
        }
        .ok_or_else(|| {
            self.fault_at(function, instruction, VmError::UnsupportedOpcode(op.opcode))
        })?;
        self.at(function, instruction, write_reg(registers, op.a, result))
    }

    pub(crate) fn execute_finite_for_init(
        &self,
        function: usize,
        instruction: usize,
        registers: &mut [Value],
        op: &Instruction,
    ) -> Result<(), VmFault> {
        let iterable = self.at(function, instruction, read_reg(registers, op.b))?;
        let len = finite_iter_len(iterable).ok_or_else(|| {
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
        let item = finite_iter_item(iterable, iterator).ok_or_else(|| {
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

    pub(crate) fn execute_panic(
        &self,
        function_index: usize,
        instruction_index: usize,
        function: &tune_bytecode::function::BytecodeFunction,
        registers: &[Value],
        op: &Instruction,
    ) -> VmFault {
        let Some(site) = function.panic_sites.get(op.a as usize) else {
            return self.fault_at(
                function_index,
                instruction_index,
                VmError::PanicSiteOutOfBounds,
            );
        };
        let message = if let Some(arg) = site.args.first() {
            read_reg(registers, *arg).map_or_else(
                |_| format!("panic({} arg(s))", site.args.len()),
                panic_message,
            )
        } else {
            "panic".to_owned()
        };
        self.fault_at(
            function_index,
            instruction_index,
            VmError::Panic(TunePanic { message }),
        )
    }
}

fn finite_iter_len(iterable: Value) -> Option<i64> {
    match iterable {
        Value::Sequence(values) => i64::try_from(values.len()).ok(),
        value => value_range(value).and_then(range_len),
    }
}

fn finite_iter_item(iterable: Value, iterator: i64) -> Option<Value> {
    match iterable {
        Value::Sequence(values) => {
            let index = usize::try_from(iterator).ok()?;
            values.get(index).cloned()
        }
        value => range_item(value_range(value)?, iterator).map(|item| item.value),
    }
}

fn panic_message(value: Value) -> String {
    match value {
        Value::String(message) => message,
        Value::Int(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Size(value) => value.to_string(),
        Value::Byte(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Unit => "unit".to_owned(),
        _ => format!("{value:?}"),
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
