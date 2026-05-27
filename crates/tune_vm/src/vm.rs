use tune_bytecode::{
    Opcode,
    artifact::{BytecodeArtifact, BytecodeConst},
};
use tune_runtime::value::Value;

pub struct Vm {
    pub artifact: BytecodeArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    MissingEntry,
    RegisterOutOfBounds,
    ConstantOutOfBounds,
    UnsupportedOpcode(Opcode),
}

impl Vm {
    pub fn new(artifact: BytecodeArtifact) -> Self {
        Self { artifact }
    }

    pub fn run_entry(&mut self) -> Result<Value, VmError> {
        // v0: dense Rust match dispatch. Optimized VM can add superinstructions later.
        let entry = self.artifact.entry_function.ok_or(VmError::MissingEntry)? as usize;
        let function = self
            .artifact
            .functions
            .get(entry)
            .ok_or(VmError::MissingEntry)?;
        let mut registers = vec![Value::Unit; function.register_count as usize];
        let mut locals = vec![Value::Unit; function.local_count as usize];
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
                Opcode::AddInt => {
                    let left = read_reg(&registers, instruction.b)?;
                    let right = read_reg(&registers, instruction.c)?;
                    let (Value::Int(left), Value::Int(right)) = (left, right) else {
                        return Err(VmError::UnsupportedOpcode(Opcode::AddInt));
                    };
                    write_reg(&mut registers, instruction.a, Value::Int(left + right))?;
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
        }
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
