use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::{Arc, atomic::AtomicU64};
use std::thread::JoinHandle;

use tune_bytecode::{artifact::BytecodeArtifact, validate_artifact};
use tune_runtime::{
    task::{TaskExecutionMode, TaskId},
    value::Value,
};

use crate::{VmError, VmFault, resource_table::SharedResourceTable};

pub struct Vm {
    pub artifact: BytecodeArtifact,
    pub task_execution: TaskExecutionMode,
    pub(crate) host_executors: Vec<Option<tune_host::HostExecutor>>,
    pub(crate) host_authorities: Vec<Vec<tune_host::Authority>>,
    pub(crate) host_resource_types: Vec<crate::VmHostResourceType>,
    pub(crate) granted_authorities: HashSet<tune_host::Authority>,
    pub(crate) resources: SharedResourceTable,
    pub(crate) next_state_id: Arc<AtomicU64>,
    pub(crate) tasks: RefCell<Vec<VmTask>>,
    pub(crate) task_context: bool,
}

#[derive(Debug)]
pub(crate) enum VmTask {
    Pending {
        id: TaskId,
        function: u32,
        args: Vec<Value>,
    },
    Running {
        id: TaskId,
        handle: Option<JoinHandle<Result<Value, VmFault>>>,
    },
    Ready {
        value: Value,
    },
}

impl Vm {
    pub fn new(artifact: BytecodeArtifact) -> Self {
        Self {
            artifact,
            task_execution: TaskExecutionMode::Parallel,
            host_executors: Vec::new(),
            host_authorities: Vec::new(),
            host_resource_types: Vec::new(),
            granted_authorities: HashSet::new(),
            resources: SharedResourceTable::default(),
            next_state_id: Arc::new(AtomicU64::new(0)),
            tasks: RefCell::new(Vec::new()),
            task_context: false,
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

    #[must_use]
    pub fn with_host_authority_slots(
        mut self,
        authorities: impl IntoIterator<Item = Vec<tune_host::Authority>>,
    ) -> Self {
        self.host_authorities = authorities.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_host_resource_types(
        mut self,
        resources: impl IntoIterator<Item = crate::VmHostResourceType>,
    ) -> Self {
        self.host_resource_types = resources.into_iter().collect();
        self
    }

    #[must_use]
    pub fn with_authorities(
        mut self,
        authorities: impl IntoIterator<Item = tune_host::Authority>,
    ) -> Self {
        self.granted_authorities = authorities.into_iter().collect();
        self
    }

    pub(crate) fn task_vm(&self) -> Self {
        Self {
            artifact: self.artifact.clone(),
            task_execution: self.task_execution,
            host_executors: self.host_executors.clone(),
            host_authorities: self.host_authorities.clone(),
            host_resource_types: self.host_resource_types.clone(),
            granted_authorities: self.granted_authorities.clone(),
            resources: self.resources.clone(),
            next_state_id: Arc::clone(&self.next_state_id),
            tasks: RefCell::new(Vec::new()),
            task_context: true,
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

    pub fn cleanup_resources(&mut self) -> Result<(), VmError> {
        self.resources
            .cleanup()
            .map_err(|message| VmError::HostCallFailed { message })
    }
}
