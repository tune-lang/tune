use tune_db::SourceMap;
use tune_runtime::Value;

use crate::{EngineError, diagnostic_from_vm_fault_with_sources};

pub struct Runtime {
    vm: tune_vm::Vm,
    sources: SourceMap,
}

impl Runtime {
    #[must_use]
    pub(crate) const fn new(vm: tune_vm::Vm, sources: SourceMap) -> Self {
        Self { vm, sources }
    }

    pub fn run_entry(&mut self) -> Result<Value, EngineError> {
        self.vm.run_entry().map_err(|fault| {
            EngineError::Diagnostics(vec![diagnostic_from_vm_fault_with_sources(
                &fault,
                &self.sources,
            )])
        })
    }
}
