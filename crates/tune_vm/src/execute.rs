use tune_bytecode::{Opcode, artifact::BytecodeConst};
use tune_runtime::{
    task::TaskJoinOutcome,
    value::{CallableValue, RangeItemKind, RangeValue, StructFields, Value},
};

use crate::execute_support::{read_reg, runtime_variant, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_function(
        &self,
        function_index: usize,
        args: Vec<Value>,
    ) -> Result<Value, VmFault> {
        let function = self
            .artifact
            .functions
            .get(function_index)
            .ok_or_else(|| VmFault::new(VmError::FunctionOutOfBounds, None))?;
        let mut registers = vec![Value::Unit; function.register_count as usize];
        let mut locals = vec![Value::Unit; function.local_count as usize];
        if args.len() != function.param_count as usize
            || function.param_count > function.local_count
        {
            return Err(self.function_fault(function_index, VmError::ArityMismatch));
        }
        for (slot, arg) in args.into_iter().enumerate() {
            locals[slot] = arg;
        }
        let mut ip = 0;
        while let Some(instruction) = function.instructions.get(ip) {
            match instruction.opcode {
                Opcode::LoadConst => {
                    let value = match self
                        .artifact
                        .constants
                        .get(instruction.b as usize)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::ConstantOutOfBounds)
                        })? {
                        BytecodeConst::Int(value) => Value::Int(*value),
                        BytecodeConst::Bool(value) => Value::Bool(*value),
                        BytecodeConst::String(value) => Value::String(value.clone()),
                    };
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, value),
                    )?;
                }
                Opcode::LoadLocal => {
                    let value = self.at(function_index, ip, read_reg(&locals, instruction.b))?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, value),
                    )?;
                }
                Opcode::StoreLocal => {
                    let value = self.at(function_index, ip, read_reg(&registers, instruction.b))?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut locals, instruction.a, value),
                    )?;
                }
                Opcode::Move => {
                    let value = self.at(function_index, ip, read_reg(&registers, instruction.b))?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, value),
                    )?;
                }
                Opcode::SeqBuild
                | Opcode::SeqPush
                | Opcode::SeqGetChecked
                | Opcode::SeqGetUnchecked
                | Opcode::SeqSetChecked
                | Opcode::SeqSetUnchecked => {
                    self.execute_sequence(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::StructConstruct => {
                    let site = function
                        .struct_sites
                        .get(instruction.b as usize)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::StructSiteOutOfBounds)
                        })?;
                    let max_field = site
                        .fields
                        .iter()
                        .map(|field| field.field)
                        .max()
                        .unwrap_or(0);
                    let mut fields = vec![Value::Unit; max_field as usize + 1];
                    for field in &site.fields {
                        fields[field.field as usize] =
                            self.at(function_index, ip, read_reg(&registers, field.value))?;
                    }
                    let state = self.at(function_index, ip, self.alloc_state(site.state))?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(
                            &mut registers,
                            instruction.a,
                            Value::Struct {
                                owner: site.owner,
                                state,
                                fields: StructFields::new(fields),
                            },
                        ),
                    )?;
                }
                Opcode::FieldGet => {
                    match self.at(function_index, ip, read_reg(&registers, instruction.b))? {
                        Value::Struct { fields, .. } => {
                            let value = fields.get(instruction.c as usize).ok_or_else(|| {
                                self.fault_at(function_index, ip, VmError::RegisterOutOfBounds)
                            })?;
                            self.at(
                                function_index,
                                ip,
                                write_reg(&mut registers, instruction.a, value),
                            )?;
                        }
                        _ => {
                            return Err(self.fault_at(
                                function_index,
                                ip,
                                VmError::UnsupportedOpcode(Opcode::FieldGet),
                            ));
                        }
                    }
                }
                Opcode::FieldSet => {
                    match self.at(function_index, ip, read_reg(&registers, instruction.a))? {
                        Value::Struct { fields, .. } => {
                            let value =
                                self.at(function_index, ip, read_reg(&registers, instruction.c))?;
                            fields.set(instruction.b as usize, value).ok_or_else(|| {
                                self.fault_at(function_index, ip, VmError::RegisterOutOfBounds)
                            })?;
                        }
                        _ => {
                            return Err(self.fault_at(
                                function_index,
                                ip,
                                VmError::UnsupportedOpcode(Opcode::FieldSet),
                            ));
                        }
                    }
                }
                Opcode::AddInt => {
                    let left = self.at(function_index, ip, read_reg(&registers, instruction.b))?;
                    let right = self.at(function_index, ip, read_reg(&registers, instruction.c))?;
                    let (Value::Int(left), Value::Int(right)) = (left, right) else {
                        return Err(self.fault_at(
                            function_index,
                            ip,
                            VmError::UnsupportedOpcode(Opcode::AddInt),
                        ));
                    };
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, Value::Int(left + right)),
                    )?;
                }
                Opcode::RangeExclusiveInt | Opcode::RangeInclusiveInt => {
                    let start = self.at(function_index, ip, read_reg(&registers, instruction.b))?;
                    let end = self.at(function_index, ip, read_reg(&registers, instruction.c))?;
                    let Some((start, end, item)) = range_parts(start, end) else {
                        return Err(self.fault_at(
                            function_index,
                            ip,
                            VmError::UnsupportedOpcode(instruction.opcode),
                        ));
                    };
                    self.at(
                        function_index,
                        ip,
                        write_reg(
                            &mut registers,
                            instruction.a,
                            Value::Range(RangeValue {
                                start,
                                end,
                                inclusive: instruction.opcode == Opcode::RangeInclusiveInt,
                                item,
                            }),
                        ),
                    )?;
                }
                Opcode::NegInt | Opcode::NotBool => {
                    self.execute_unary(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::GreaterInt
                | Opcode::EqualInt
                | Opcode::NotEqualInt
                | Opcode::LessInt
                | Opcode::LessEqualInt
                | Opcode::GreaterEqualInt => {
                    self.execute_int_comparison(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::CallDirect => {
                    let call_site =
                        function
                            .call_sites
                            .get(instruction.b as usize)
                            .ok_or_else(|| {
                                self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds)
                            })?;
                    let args = call_site
                        .args
                        .iter()
                        .map(|arg| self.at(function_index, ip, read_reg(&registers, *arg)))
                        .collect::<Result<Vec<_>, _>>()?;
                    let value = self.execute_function(call_site.function as usize, args)?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, value),
                    )?;
                }
                Opcode::CallableValue => {
                    let site = function
                        .callable_sites
                        .get(instruction.b as usize)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds)
                        })?;
                    let captures = site
                        .captures
                        .iter()
                        .map(|capture| self.at(function_index, ip, read_reg(&registers, *capture)))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(
                            &mut registers,
                            instruction.a,
                            Value::Callable(CallableValue {
                                function: site.function,
                                captures,
                            }),
                        ),
                    )?;
                }
                Opcode::CallBound => {
                    let Value::Callable(callable) =
                        self.at(function_index, ip, read_reg(&registers, instruction.c))?
                    else {
                        return Err(self.fault_at(
                            function_index,
                            ip,
                            VmError::UnsupportedOpcode(Opcode::CallBound),
                        ));
                    };
                    let site = function
                        .bound_call_sites
                        .get(instruction.b as usize)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds)
                        })?;
                    let args = site
                        .args
                        .iter()
                        .map(|arg| self.at(function_index, ip, read_reg(&registers, *arg)))
                        .collect::<Result<Vec<_>, _>>()?;
                    let value = self.execute_function(
                        callable.function as usize,
                        callable.captures.into_iter().chain(args).collect(),
                    )?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, value),
                    )?;
                }
                Opcode::VariantConstruct => {
                    let variant_site = function
                        .variant_sites
                        .get(instruction.b as usize)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds)
                        })?;
                    let fields = variant_site
                        .args
                        .iter()
                        .map(|arg| self.at(function_index, ip, read_reg(&registers, *arg)))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(
                            &mut registers,
                            instruction.a,
                            Value::Variant {
                                variant: runtime_variant(variant_site.variant),
                                fields,
                                propagation_frames: Vec::new(),
                            },
                        ),
                    )?;
                }
                Opcode::VariantField => {
                    match self.at(function_index, ip, read_reg(&registers, instruction.b))? {
                        Value::Variant { fields, .. } => {
                            let value =
                                fields.get(instruction.c as usize).cloned().ok_or_else(|| {
                                    self.fault_at(function_index, ip, VmError::RegisterOutOfBounds)
                                })?;
                            self.at(
                                function_index,
                                ip,
                                write_reg(&mut registers, instruction.a, value),
                            )?;
                        }
                        _ => {
                            return Err(self.fault_at(
                                function_index,
                                ip,
                                VmError::UnsupportedOpcode(Opcode::VariantField),
                            ));
                        }
                    }
                }
                Opcode::ResultPropagate => {
                    let result =
                        self.at(function_index, ip, read_reg(&registers, instruction.b))?;
                    if let Some(value) = self.propagate_result(
                        function_index,
                        ip,
                        &mut registers,
                        instruction.a,
                        result,
                    )? {
                        return Ok(value);
                    }
                }
                Opcode::SpawnTask => {
                    let value = self.at(function_index, ip, read_reg(&registers, instruction.b))?;
                    let task = self.push_ready_task(value);
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, task),
                    )?;
                }
                Opcode::TaskJoin => {
                    match self.at(function_index, ip, read_reg(&registers, instruction.b))? {
                        Value::Task(handle) => match self.join_task(handle.0) {
                            Some(TaskJoinOutcome::Ready(value)) => {
                                self.at(
                                    function_index,
                                    ip,
                                    write_reg(&mut registers, instruction.a, value),
                                )?;
                            }
                            Some(TaskJoinOutcome::Pending(_)) => {
                                return Err(self.fault_at(
                                    function_index,
                                    ip,
                                    VmError::UnsupportedOpcode(Opcode::TaskJoin),
                                ));
                            }
                            Some(TaskJoinOutcome::UnrecoverablePanic(_)) => {
                                return Err(self.fault_at(
                                    function_index,
                                    ip,
                                    VmError::UnsupportedOpcode(Opcode::TaskJoin),
                                ));
                            }
                            None => {
                                return Err(self.fault_at(
                                    function_index,
                                    ip,
                                    VmError::RegisterOutOfBounds,
                                ));
                            }
                        },
                        _ => {
                            return Err(self.fault_at(
                                function_index,
                                ip,
                                VmError::UnsupportedOpcode(Opcode::TaskJoin),
                            ));
                        }
                    }
                }
                Opcode::Jump => {
                    ip = instruction.a as usize;
                    continue;
                }
                Opcode::JumpIfFalse => {
                    let condition =
                        self.at(function_index, ip, read_reg(&registers, instruction.a))?;
                    if matches!(condition, Value::Bool(false)) {
                        ip = instruction.b as usize;
                        continue;
                    }
                    if !matches!(condition, Value::Bool(true)) {
                        return Err(self.fault_at(
                            function_index,
                            ip,
                            VmError::UnsupportedOpcode(Opcode::JumpIfFalse),
                        ));
                    }
                }
                Opcode::MatchVariant => {
                    let Value::Variant { variant, .. } =
                        self.at(function_index, ip, read_reg(&registers, instruction.a))?
                    else {
                        return Err(self.fault_at(
                            function_index,
                            ip,
                            VmError::UnsupportedOpcode(Opcode::MatchVariant),
                        ));
                    };
                    let match_site = function
                        .match_sites
                        .get(instruction.b as usize)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds)
                        })?;
                    if let Some(arm) = match_site
                        .arms
                        .iter()
                        .find(|arm| runtime_variant(arm.variant) == variant)
                    {
                        ip = arm.target as usize;
                        continue;
                    }
                    if instruction.c == u32::MAX {
                        return Err(self.fault_at(
                            function_index,
                            ip,
                            VmError::UnsupportedOpcode(Opcode::MatchVariant),
                        ));
                    }
                    ip = instruction.c as usize;
                    continue;
                }
                Opcode::FiniteForInit => {
                    self.execute_finite_for_init(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::FiniteForNext => {
                    ip = self.execute_finite_for_next(
                        function_index,
                        ip,
                        function,
                        &mut registers,
                        instruction,
                    )?;
                    continue;
                }
                Opcode::Panic => {
                    return Err(self.execute_panic(
                        function_index,
                        ip,
                        function,
                        &registers,
                        instruction,
                    ));
                }
                Opcode::Return => {
                    if instruction.b == 0 {
                        return Ok(Value::Unit);
                    }
                    return self.at(function_index, ip, read_reg(&registers, instruction.a));
                }
                Opcode::Nop => {}
                other => {
                    return Err(self.fault_at(
                        function_index,
                        ip,
                        VmError::UnsupportedOpcode(other),
                    ));
                }
            }
            ip += 1;
        }
        Ok(Value::Unit)
    }
}

fn range_parts(start: Value, end: Value) -> Option<(i128, i128, RangeItemKind)> {
    match (start, end) {
        (Value::Int(start), Value::Int(end)) => {
            Some((i128::from(start), i128::from(end), RangeItemKind::Int))
        }
        (Value::Size(start), Value::Size(end)) => {
            Some((i128::from(start), i128::from(end), RangeItemKind::Size))
        }
        _ => None,
    }
}
