use std::cell::{Cell, RefCell};

use tune_bytecode::{artifact::BytecodeArtifact, validate_artifact};
use tune_runtime::{
    task::{TaskExecutionMode, TaskId, TaskJoinOutcome},
    value::Value,
};

use crate::{VmError, VmFault};

pub struct Vm {
    pub artifact: BytecodeArtifact,
    pub task_execution: TaskExecutionMode,
    pub(crate) host_executors: Vec<Option<tune_host::HostExecutor>>,
    pub(crate) next_state_id: Cell<u64>,
    pub(crate) tasks: RefCell<Vec<VmTask>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum VmTask {
    Pending {
        id: TaskId,
        function: u32,
        args: Vec<Value>,
    },
    Ready {
        value: Value,
    },
}

impl VmTask {
    pub(crate) fn join(self) -> TaskJoinOutcome {
        match self {
            Self::Pending { id, .. } => TaskJoinOutcome::Pending(id),
            Self::Ready { value } => TaskJoinOutcome::Ready(value),
        }
    }
}

impl Vm {
    pub fn new(artifact: BytecodeArtifact) -> Self {
        Self {
            artifact,
            task_execution: TaskExecutionMode::DeferredUntilJoin,
            host_executors: Vec::new(),
            next_state_id: Cell::new(0),
            tasks: RefCell::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn with_task_execution(mut self, mode: TaskExecutionMode) -> Self {
        self.task_execution = mode;
        self
    }

    #[must_use]
    pub fn with_host_executors(
        mut self,
        executors: impl IntoIterator<Item = tune_host::HostExecutor>,
    ) -> Self {
        self.host_executors = executors.into_iter().map(Some).collect();
        self
    }

    #[must_use]
    pub fn with_host_executor_slots(
        mut self,
        executors: Vec<Option<tune_host::HostExecutor>>,
    ) -> Self {
        self.host_executors = executors;
        self
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
