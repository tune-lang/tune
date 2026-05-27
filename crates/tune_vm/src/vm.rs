use tune_bytecode::{Opcode, artifact::BytecodeArtifact};
use tune_runtime::value::Value;

pub struct Vm {
    pub artifact: BytecodeArtifact,
}

impl Vm {
    pub fn new(artifact: BytecodeArtifact) -> Self {
        Self { artifact }
    }

    pub fn run_main(&mut self) -> Value {
        // v0: dense Rust match dispatch. Optimized VM can add superinstructions later.
        Value::Unit
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
