use std::cell::{Cell, RefCell};

use tune_bytecode::{artifact::BytecodeArtifact, validate_artifact};
use tune_runtime::{task::Task, value::Value};

use crate::{VmError, VmFault};

pub struct Vm {
    pub artifact: BytecodeArtifact,
    pub(crate) next_state_id: Cell<u64>,
    pub(crate) tasks: RefCell<Vec<Task>>,
}

impl Vm {
    pub fn new(artifact: BytecodeArtifact) -> Self {
        Self {
            artifact,
            next_state_id: Cell::new(0),
            tasks: RefCell::new(Vec::new()),
        }
    }

    pub fn run_entry(&mut self) -> Result<Value, VmFault> {
        // v0: dense Rust match dispatch. Optimized VM can add superinstructions later.
        validate_artifact(&self.artifact)
            .map_err(VmError::InvalidBytecode)
            .map_err(|error| VmFault::new(error, None))?;
        let entry = self
            .artifact
            .entry_function
            .ok_or_else(|| VmFault::new(VmError::MissingEntry, None))? as usize;
        self.execute_function(entry, Vec::new())
    }
}
