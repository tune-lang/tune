use tune_bytecode::{Opcode, artifact::BytecodeConst, function::BytecodeCaptureMode};
use tune_runtime::value::{
    CallableValue, CaptureStorageMode, CapturedValue, RangeItemKind, RangeValue, Value,
};

use crate::execute_support::{read_reg, write_reg};
use crate::{Vm, VmError, VmFault};

impl Vm {
    pub(crate) fn execute_function(
        &self,
        function_index: usize,
        args: Vec<Value>,
    ) -> Result<Value, VmFault> {
        self.execute_function_with_capture_cells(function_index, args, None, &[], 0)
    }

    pub(crate) fn execute_task_function(
        &self,
        function_index: usize,
        args: Vec<Value>,
    ) -> Result<Value, VmFault> {
        self.execute_function_with_capture_cells(function_index, args, None, &[], 0)
    }

    fn execute_function_with_capture_cells(
        &self,
        function_index: usize,
        args: Vec<Value>,
        initial_locals: Option<Vec<Value>>,
        capture_cells: &[CapturedValue],
        capture_count: usize,
    ) -> Result<Value, VmFault> {
        let function = self
            .artifact
            .functions
            .get(function_index)
            .ok_or_else(|| VmFault::new(VmError::FunctionOutOfBounds, None))?;
        let mut registers = vec![Value::Unit; function.register_count as usize];
        let mut locals =
            initial_locals.unwrap_or_else(|| vec![Value::Unit; function.local_count as usize]);
        locals.resize(function.local_count as usize, Value::Unit);
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
                        BytecodeConst::Float(value) => Value::Float(*value),
                        BytecodeConst::Size(value) => Value::Size(*value),
                        BytecodeConst::Byte(value) => Value::Byte(*value),
                        BytecodeConst::Bool(value) => Value::Bool(*value),
                        BytecodeConst::None => Value::None,
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
                Opcode::TupleBuild => {
                    self.execute_tuple_build(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::StringBuild => {
                    self.execute_string_build(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::StringLen => {
                    self.execute_string_len(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::StringGet => {
                    self.execute_string_get(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::StructConstruct => {
                    self.execute_struct_construct(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::FieldGet => {
                    self.execute_field_get(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::StructIs => {
                    self.execute_struct_is(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::FieldSet => {
                    self.execute_field_set(function_index, ip, &registers, instruction)?;
                }
                Opcode::AddInt
                | Opcode::SubInt
                | Opcode::MulInt
                | Opcode::DivInt
                | Opcode::RemInt
                | Opcode::BitAndInt
                | Opcode::BitOrInt
                | Opcode::BitXorInt
                | Opcode::ShiftLeftInt
                | Opcode::ShiftRightInt
                | Opcode::AddFloat
                | Opcode::SubFloat
                | Opcode::MulFloat
                | Opcode::DivFloat
                | Opcode::AddSizeChecked
                | Opcode::SubSizeChecked
                | Opcode::MulSizeChecked
                | Opcode::DivSize
                | Opcode::RemSize
                | Opcode::AddByteWrap
                | Opcode::SubByteWrap
                | Opcode::MulByteWrap
                | Opcode::DivByte
                | Opcode::RemByte
                | Opcode::BitNotByte
                | Opcode::BitAndByte
                | Opcode::BitOrByte
                | Opcode::BitXorByte
                | Opcode::ShiftLeftByte
                | Opcode::ShiftRightByte => {
                    self.execute_add(function_index, ip, &mut registers, instruction)?;
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
                Opcode::NegInt | Opcode::NotBool | Opcode::BitNotInt | Opcode::NoneCheck => {
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
                Opcode::GreaterFloat
                | Opcode::EqualFloat
                | Opcode::NotEqualFloat
                | Opcode::LessFloat
                | Opcode::LessEqualFloat
                | Opcode::GreaterEqualFloat => {
                    self.execute_float_comparison(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::GreaterSize
                | Opcode::EqualSize
                | Opcode::NotEqualSize
                | Opcode::LessSize
                | Opcode::LessEqualSize
                | Opcode::GreaterEqualSize => {
                    self.execute_size_comparison(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::GreaterByte
                | Opcode::EqualByte
                | Opcode::NotEqualByte
                | Opcode::LessByte
                | Opcode::LessEqualByte
                | Opcode::GreaterEqualByte => {
                    self.execute_byte_comparison(function_index, ip, &mut registers, instruction)?;
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
                        .map(|capture| {
                            self.at(function_index, ip, read_reg(&registers, capture.register))
                                .and_then(|value| match capture.mode {
                                    BytecodeCaptureMode::Reference => {
                                        Ok(CapturedValue::new(value, CaptureStorageMode::Reference))
                                    }
                                    BytecodeCaptureMode::PrivateSnapshot => self
                                        .at(function_index, ip, self.capture_snapshot(&value))
                                        .map(|snapshot| {
                                            CapturedValue::new(
                                                snapshot,
                                                CaptureStorageMode::PrivateSnapshot,
                                            )
                                        }),
                                })
                        })
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
                    let capture_count = callable.captures.len();
                    let capture_cells = callable.captures;
                    let captured_args = capture_cells
                        .iter()
                        .map(CapturedValue::get)
                        .collect::<Vec<_>>();
                    let value = self.execute_function_with_capture_cells(
                        callable.function as usize,
                        captured_args.into_iter().chain(args).collect(),
                        None,
                        &capture_cells,
                        capture_count,
                    )?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, value),
                    )?;
                }
                Opcode::CallHost => {
                    let site = function
                        .host_call_sites
                        .get(instruction.b as usize)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::CallSiteOutOfBounds)
                        })?;
                    let executor = self
                        .host_executors
                        .get(site.symbol.0 as usize)
                        .and_then(Option::as_ref)
                        .ok_or_else(|| {
                            self.fault_at(function_index, ip, VmError::HostSymbolOutOfBounds)
                        })?;
                    if let Some(required) = self.host_authorities.get(site.symbol.0 as usize) {
                        for authority in required {
                            if !self.granted_authorities.contains(authority) {
                                return Err(self.fault_at(
                                    function_index,
                                    ip,
                                    VmError::MissingHostAuthority {
                                        authority: authority.0.clone(),
                                    },
                                ));
                            }
                        }
                    }
                    let args = site
                        .args
                        .iter()
                        .map(|arg| self.at(function_index, ip, read_reg(&registers, *arg)))
                        .collect::<Result<Vec<_>, _>>()?;
                    let value = executor.call(&args).map_err(|error| {
                        self.fault_at(
                            function_index,
                            ip,
                            VmError::HostCallFailed {
                                message: error.message,
                            },
                        )
                    })?;
                    self.at(
                        function_index,
                        ip,
                        write_reg(&mut registers, instruction.a, value),
                    )?;
                }
                Opcode::VariantConstruct => {
                    self.execute_variant_construct(
                        function_index,
                        ip,
                        &mut registers,
                        instruction,
                    )?;
                }
                Opcode::VariantField => {
                    self.execute_variant_field(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::TupleField => {
                    self.execute_tuple_field(function_index, ip, &mut registers, instruction)?;
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
                        write_back_captures(&locals, capture_cells, capture_count);
                        return Ok(value);
                    }
                }
                Opcode::SpawnTask => {
                    self.execute_spawn_task(function_index, ip, &mut registers, instruction)?;
                }
                Opcode::TaskJoin => {
                    self.execute_task_join(function_index, ip, &mut registers, instruction)?;
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
                    ip = self.execute_match_variant(function_index, ip, &registers, instruction)?;
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
                        write_back_captures(&locals, capture_cells, capture_count);
                        return Ok(Value::Unit);
                    }
                    let value = self.at(function_index, ip, read_reg(&registers, instruction.a))?;
                    write_back_captures(&locals, capture_cells, capture_count);
                    return Ok(value);
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
        write_back_captures(&locals, capture_cells, capture_count);
        Ok(Value::Unit)
    }
}

fn write_back_captures(locals: &[Value], capture_cells: &[CapturedValue], capture_count: usize) {
    for (index, cell) in capture_cells.iter().take(capture_count).enumerate() {
        if cell.mode() != CaptureStorageMode::PrivateSnapshot {
            continue;
        }
        if let Some(value) = locals.get(index) {
            cell.set(value.clone());
        }
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
