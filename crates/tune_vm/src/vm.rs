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
            Opcode::AddInt => {}
            Opcode::AddFloat => {}
            Opcode::AddSizeChecked => {}
            Opcode::AddByteWrap => {}
            Opcode::SeqGetChecked => {}
            Opcode::SeqGetUnchecked => {}
            Opcode::FieldGet => {}
            Opcode::FieldSet => {}
            Opcode::CallDirect => {}
            Opcode::CallBound => {}
            Opcode::CallWitness => {}
            Opcode::CallHost => {}
            Opcode::ResultPropagate => {}
            Opcode::SpawnTask => {}
            Opcode::TaskJoin => {}
            Opcode::Return => {}
        }
    }
}
