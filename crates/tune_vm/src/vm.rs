use std::cell::Cell;

use tune_bytecode::{
    Opcode,
    artifact::{BytecodeArtifact, BytecodeConst},
    function::{BytecodeOwnershipPlan, BytecodeStateRepr, BytecodeStructState, BytecodeVariant},
};
use tune_runtime::{
    ownership::OwnershipPlan,
    state::{StateHandle, StateId, StateRepr},
    value::{RuntimeVariant, StructFields, Value},
};

pub struct Vm {
    pub artifact: BytecodeArtifact,
    next_state_id: Cell<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    MissingEntry,
    RegisterOutOfBounds,
    ConstantOutOfBounds,
    FunctionOutOfBounds,
    CallSiteOutOfBounds,
    StructSiteOutOfBounds,
    ArityMismatch,
    UnsupportedStructState,
    UnsupportedOpcode(Opcode),
}

impl Vm {
    pub fn new(artifact: BytecodeArtifact) -> Self {
        Self {
            artifact,
            next_state_id: Cell::new(0),
        }
    }

    pub fn run_entry(&mut self) -> Result<Value, VmError> {
        // v0: dense Rust match dispatch. Optimized VM can add superinstructions later.
        let entry = self.artifact.entry_function.ok_or(VmError::MissingEntry)? as usize;
        self.execute_function(entry, Vec::new())
    }

    fn execute_function(&self, function_index: usize, args: Vec<Value>) -> Result<Value, VmError> {
        let function = self
            .artifact
            .functions
            .get(function_index)
            .ok_or(VmError::FunctionOutOfBounds)?;
        let mut registers = vec![Value::Unit; function.register_count as usize];
        let mut locals = vec![Value::Unit; function.local_count as usize];
        if args.len() > locals.len() {
            return Err(VmError::ArityMismatch);
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
                        .ok_or(VmError::ConstantOutOfBounds)?
                    {
                        BytecodeConst::Int(value) => Value::Int(*value),
                        BytecodeConst::Bool(value) => Value::Bool(*value),
                    };
                    write_reg(&mut registers, instruction.a, value)?;
                }
                Opcode::LoadLocal => {
                    let value = read_reg(&locals, instruction.b)?;
                    write_reg(&mut registers, instruction.a, value)?;
                }
                Opcode::StoreLocal => {
                    let value = read_reg(&registers, instruction.b)?;
                    write_reg(&mut locals, instruction.a, value)?;
                }
                Opcode::Move => {
                    let value = read_reg(&registers, instruction.b)?;
                    write_reg(&mut registers, instruction.a, value)?;
                }
                Opcode::StructConstruct => {
                    let site = function
                        .struct_sites
                        .get(instruction.b as usize)
                        .ok_or(VmError::StructSiteOutOfBounds)?;
                    let max_field = site
                        .fields
                        .iter()
                        .map(|field| field.field)
                        .max()
                        .unwrap_or(0);
                    let mut fields = vec![Value::Unit; max_field as usize + 1];
                    for field in &site.fields {
                        fields[field.field as usize] = read_reg(&registers, field.value)?;
                    }
                    write_reg(
                        &mut registers,
                        instruction.a,
                        Value::Struct {
                            owner: site.owner,
                            state: self.alloc_state(site.state)?,
                            fields: StructFields::new(fields),
                        },
                    )?;
                }
                Opcode::FieldGet => match read_reg(&registers, instruction.b)? {
                    Value::Struct { fields, .. } => {
                        let value = fields
                            .get(instruction.c as usize)
                            .ok_or(VmError::RegisterOutOfBounds)?;
                        write_reg(&mut registers, instruction.a, value)?;
                    }
                    _ => return Err(VmError::UnsupportedOpcode(Opcode::FieldGet)),
                },
                Opcode::FieldSet => match read_reg(&registers, instruction.a)? {
                    Value::Struct { fields, .. } => {
                        let value = read_reg(&registers, instruction.c)?;
                        fields
                            .set(instruction.b as usize, value)
                            .ok_or(VmError::RegisterOutOfBounds)?;
                    }
                    _ => return Err(VmError::UnsupportedOpcode(Opcode::FieldSet)),
                },
                Opcode::AddInt => {
                    let left = read_reg(&registers, instruction.b)?;
                    let right = read_reg(&registers, instruction.c)?;
                    let (Value::Int(left), Value::Int(right)) = (left, right) else {
                        return Err(VmError::UnsupportedOpcode(Opcode::AddInt));
                    };
                    write_reg(&mut registers, instruction.a, Value::Int(left + right))?;
                }
                Opcode::GreaterInt => {
                    let left = read_reg(&registers, instruction.b)?;
                    let right = read_reg(&registers, instruction.c)?;
                    let (Value::Int(left), Value::Int(right)) = (left, right) else {
                        return Err(VmError::UnsupportedOpcode(Opcode::GreaterInt));
                    };
                    write_reg(&mut registers, instruction.a, Value::Bool(left > right))?;
                }
                Opcode::CallDirect => {
                    let call_site = function
                        .call_sites
                        .get(instruction.b as usize)
                        .ok_or(VmError::CallSiteOutOfBounds)?;
                    let args = call_site
                        .args
                        .iter()
                        .map(|arg| read_reg(&registers, *arg))
                        .collect::<Result<Vec<_>, _>>()?;
                    let value = self.execute_function(call_site.function as usize, args)?;
                    write_reg(&mut registers, instruction.a, value)?;
                }
                Opcode::VariantConstruct => {
                    let variant_site = function
                        .variant_sites
                        .get(instruction.b as usize)
                        .ok_or(VmError::CallSiteOutOfBounds)?;
                    let fields = variant_site
                        .args
                        .iter()
                        .map(|arg| read_reg(&registers, *arg))
                        .collect::<Result<Vec<_>, _>>()?;
                    write_reg(
                        &mut registers,
                        instruction.a,
                        Value::Variant {
                            variant: runtime_variant(variant_site.variant),
                            fields,
                        },
                    )?;
                }
                Opcode::VariantField => match read_reg(&registers, instruction.b)? {
                    Value::Variant { fields, .. } => {
                        let value = fields
                            .get(instruction.c as usize)
                            .cloned()
                            .ok_or(VmError::RegisterOutOfBounds)?;
                        write_reg(&mut registers, instruction.a, value)?;
                    }
                    _ => return Err(VmError::UnsupportedOpcode(Opcode::VariantField)),
                },
                Opcode::ResultPropagate => match read_reg(&registers, instruction.b)? {
                    Value::Variant {
                        variant: RuntimeVariant::ResultOk,
                        mut fields,
                    } if fields.len() == 1 => {
                        write_reg(&mut registers, instruction.a, fields.remove(0))?;
                    }
                    Value::Variant {
                        variant: RuntimeVariant::ResultError,
                        fields,
                    } => {
                        return Ok(Value::Variant {
                            variant: RuntimeVariant::ResultError,
                            fields,
                        });
                    }
                    _ => return Err(VmError::UnsupportedOpcode(Opcode::ResultPropagate)),
                },
                Opcode::Jump => {
                    ip = instruction.a as usize;
                    continue;
                }
                Opcode::JumpIfFalse => {
                    let condition = read_reg(&registers, instruction.a)?;
                    if matches!(condition, Value::Bool(false)) {
                        ip = instruction.b as usize;
                        continue;
                    }
                    if !matches!(condition, Value::Bool(true)) {
                        return Err(VmError::UnsupportedOpcode(Opcode::JumpIfFalse));
                    }
                }
                Opcode::MatchVariant => {
                    let Value::Variant { variant, .. } = read_reg(&registers, instruction.a)?
                    else {
                        return Err(VmError::UnsupportedOpcode(Opcode::MatchVariant));
                    };
                    let match_site = function
                        .match_sites
                        .get(instruction.b as usize)
                        .ok_or(VmError::CallSiteOutOfBounds)?;
                    if let Some(arm) = match_site
                        .arms
                        .iter()
                        .find(|arm| runtime_variant(arm.variant) == variant)
                    {
                        ip = arm.target as usize;
                        continue;
                    }
                    if instruction.c == u32::MAX {
                        return Err(VmError::UnsupportedOpcode(Opcode::MatchVariant));
                    }
                    ip = instruction.c as usize;
                    continue;
                }
                Opcode::Return => {
                    if instruction.b == 0 {
                        return Ok(Value::Unit);
                    }
                    return read_reg(&registers, instruction.a);
                }
                Opcode::Nop => {}
                other => return Err(VmError::UnsupportedOpcode(other)),
            }
            ip += 1;
        }
        Ok(Value::Unit)
    }

    fn alloc_state(&self, state: BytecodeStructState) -> Result<StateHandle, VmError> {
        if state.repr != BytecodeStateRepr::LocalHandle
            || state.ownership != BytecodeOwnershipPlan::NonAtomicRc
        {
            return Err(VmError::UnsupportedStructState);
        }
        let id = StateId(self.next_state_id.get());
        self.next_state_id
            .set(self.next_state_id.get().saturating_add(1));
        Ok(StateHandle {
            id,
            repr: runtime_state_repr(state.repr),
            ownership: runtime_ownership(state.ownership),
        })
    }

    #[allow(dead_code)]
    fn dispatch_one(&mut self, opcode: Opcode) {
        match opcode {
            Opcode::Nop => {}
            Opcode::LoadConst => {}
            Opcode::LoadLocal => {}
            Opcode::StoreLocal => {}
            Opcode::Move => {}
            Opcode::AddInt => {}
            Opcode::AddFloat => {}
            Opcode::AddSizeChecked => {}
            Opcode::AddByteWrap => {}
            Opcode::SeqBuild => {}
            Opcode::SeqPush => {}
            Opcode::SeqGetChecked => {}
            Opcode::SeqGetUnchecked => {}
            Opcode::SeqSetChecked => {}
            Opcode::SeqSetUnchecked => {}
            Opcode::FieldGet => {}
            Opcode::FieldSet => {}
            Opcode::StructConstruct => {}
            Opcode::VariantConstruct => {}
            Opcode::CallDirect => {}
            Opcode::CallBound => {}
            Opcode::CallWitness => {}
            Opcode::CallHost => {}
            Opcode::Jump => {}
            Opcode::JumpIfFalse => {}
            Opcode::MatchVariant => {}
            Opcode::FiniteForInit => {}
            Opcode::FiniteForNext => {}
            Opcode::ResultPropagate => {}
            Opcode::SpawnTask => {}
            Opcode::TaskJoin => {}
            Opcode::StringBuild => {}
            Opcode::Panic => {}
            Opcode::Return => {}
            Opcode::GreaterInt => {}
            Opcode::VariantField => {}
        }
    }
}

fn runtime_state_repr(repr: BytecodeStateRepr) -> StateRepr {
    match repr {
        BytecodeStateRepr::Inline => StateRepr::Inline,
        BytecodeStateRepr::LocalHandle => StateRepr::LocalHandle,
        BytecodeStateRepr::SharedHandle => StateRepr::SharedHandle,
        BytecodeStateRepr::HostResource => StateRepr::HostResource,
    }
}

fn runtime_ownership(ownership: BytecodeOwnershipPlan) -> OwnershipPlan {
    match ownership {
        BytecodeOwnershipPlan::Stack => OwnershipPlan::Stack,
        BytecodeOwnershipPlan::DirectDrop => OwnershipPlan::DirectDrop,
        BytecodeOwnershipPlan::NonAtomicRc => OwnershipPlan::NonAtomicRc,
        BytecodeOwnershipPlan::Cow => OwnershipPlan::Cow,
        BytecodeOwnershipPlan::SharedAtomic => OwnershipPlan::SharedAtomic,
        BytecodeOwnershipPlan::HostRetained => OwnershipPlan::HostRetained,
    }
}

fn read_reg(registers: &[Value], reg: u32) -> Result<Value, VmError> {
    registers
        .get(reg as usize)
        .cloned()
        .ok_or(VmError::RegisterOutOfBounds)
}

fn write_reg(registers: &mut [Value], reg: u32, value: Value) -> Result<(), VmError> {
    let slot = registers
        .get_mut(reg as usize)
        .ok_or(VmError::RegisterOutOfBounds)?;
    *slot = value;
    Ok(())
}

fn runtime_variant(variant: BytecodeVariant) -> RuntimeVariant {
    match variant {
        BytecodeVariant::ResultOk => RuntimeVariant::ResultOk,
        BytecodeVariant::ResultError => RuntimeVariant::ResultError,
        BytecodeVariant::Other { owner, index } => RuntimeVariant::Other { owner, index },
    }
}
