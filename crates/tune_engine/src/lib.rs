mod diagnostics;
mod executable;
mod host;
mod imports;
mod imports_closure;
mod imports_diagnostics;
mod imports_internalize;
mod imports_remap;
mod imports_shapes;
mod meta;
mod paths;
mod profile;
mod project;
mod project_sources;
mod reachable;
mod reports;
mod runtime;

pub use diagnostics::{
    diagnostic_from_result_error, diagnostic_from_result_error_with_sources,
    diagnostic_from_vm_fault, diagnostic_from_vm_fault_with_sources,
    diagnostics_from_runtime_value, diagnostics_from_runtime_value_with_sources,
};
pub use host::{
    EngineHostResourceType, EngineHostSymbol, EngineHostSymbolId, EngineHostValueType,
    EngineResourceTypeId, HostRegistration,
};
pub use profile::{
    BytecodeQuality, IrQuality, OpcodeCount, OptimizerQuality, PlanQuality, ProfileReport,
    StageTiming,
};
pub use project_sources::ProjectPackageSources;
pub use reports::{
    CheckReport, CompileReport, EngineError, EntryPoint, ExecutableReport, ProjectEntry,
    ProjectHandle, SourceId,
};
pub use runtime::Runtime;

use executable::executable_from_compile;
use tune_db::TuneDb;
use tune_diagnostics::{Diagnostic, Severity};
use tune_host::Authority;
use tune_host::module::HostModule;
use tune_runtime::TaskExecutionMode;
use tune_runtime::value::Value;

pub struct Tune {
    db: TuneDb,
    hosts: host::HostRegistry,
    authorities: Vec<Authority>,
    task_execution: TaskExecutionMode,
    projects: Vec<dyno_project::manifest::Manifest>,
    project_sources: Vec<project_sources::ProjectSourceSet>,
}

impl Default for Tune {
    fn default() -> Self {
        Self {
            db: TuneDb::default(),
            hosts: host::HostRegistry::default(),
            authorities: Vec::new(),
            task_execution: TaskExecutionMode::Parallel,
            projects: Vec::new(),
            project_sources: Vec::new(),
        }
    }
}

impl Tune {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Option<SourceId> {
        self.db.add_file(path, text)
    }

    #[must_use]
    pub fn check_source(&self, file: SourceId) -> Option<CheckReport> {
        let linked = imports::link_entry_imports(&self.db, file, &self.hosts)?;
        let resolved = tune_resolve::resolve_module(&linked.module);
        let shape = tune_shape::analyze_module(&linked.module, &resolved);
        let diagnostics = linked
            .parsed
            .iter()
            .flat_map(|parsed| parsed.diagnostics.iter())
            .chain(linked.diagnostics.iter())
            .chain(resolved.diagnostics.iter())
            .chain(
                shape
                    .iter()
                    .flat_map(|analysis| analysis.diagnostics.iter()),
            )
            .cloned()
            .collect();
        Some(CheckReport {
            file,
            diagnostics,
            module: linked.module,
            resolved,
            shape,
        })
    }

    pub fn check_text(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Option<CheckReport> {
        let file = self.add_source(path, text)?;
        self.check_source(file)
    }

    pub fn compile_source(&self, file: SourceId) -> Result<CompileReport, EngineError> {
        let check = self
            .check_source(file)
            .ok_or(EngineError::FileNotFound(file))?;
        let module_plan =
            tune_plan::lower_analyzed_module_to_plan(&check.module, &check.resolved, &check.shape);

        Ok(CompileReport { check, module_plan })
    }

    pub fn compile_text(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<CompileReport, EngineError> {
        let file = self
            .add_source(path, text)
            .ok_or(EngineError::AllocationLimit)?;
        self.compile_source(file)
    }

    pub fn executable_text(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<ExecutableReport, EngineError> {
        let file = self
            .add_source(path, text)
            .ok_or(EngineError::AllocationLimit)?;
        self.executable_source(file)
    }

    pub fn run_text(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<Value, EngineError> {
        let file = self
            .add_source(path, text)
            .ok_or(EngineError::AllocationLimit)?;
        self.run_source(file)
    }

    pub fn run_source(&self, file: SourceId) -> Result<Value, EngineError> {
        self.run_entry(EntryPoint::Source(file))
    }

    pub fn run_entry(&self, entry: EntryPoint) -> Result<Value, EngineError> {
        let executable = self.executable_entry(entry)?;
        self.runtime(executable).run_entry()
    }

    pub fn executable_source(&self, file: SourceId) -> Result<ExecutableReport, EngineError> {
        self.executable_entry(EntryPoint::Source(file))
    }

    pub fn executable_entry(&self, entry: EntryPoint) -> Result<ExecutableReport, EngineError> {
        let EntryPoint::Source(file) = entry;
        let compile = self.compile_source(file)?;
        executable_from_compile(compile)
    }

    #[must_use]
    pub fn runtime(&self, executable: ExecutableReport) -> Runtime {
        let vm = tune_vm::Vm::new(executable.bytecode)
            .with_task_execution(self.task_execution)
            .with_host_executor_slots(self.hosts.executors())
            .with_host_authority_slots(self.hosts.authorities())
            .with_host_resource_types(self.hosts.vm_resource_types())
            .with_host_value_types(executable.host_value_types.clone())
            .with_authorities(self.authorities.clone());
        Runtime::new(vm, self.db.sources().clone())
    }

    pub fn register_host(&mut self, host: &impl tune_host::Host) -> HostRegistration {
        self.hosts.register(host)
    }

    pub fn register_std(&mut self) -> HostRegistration {
        self.register_host(&tune_std::host())
    }

    #[must_use]
    pub fn host_modules(&self) -> &[HostModule] {
        self.hosts.modules()
    }

    #[must_use]
    pub fn host_symbols(&self) -> &[EngineHostSymbol] {
        self.hosts.symbols()
    }

    #[must_use]
    pub fn host_resource_types(&self) -> &[EngineHostResourceType] {
        self.hosts.resources()
    }

    #[must_use]
    pub fn host_value_types(&self) -> &[EngineHostValueType] {
        self.hosts.values()
    }

    #[must_use]
    pub fn host_resource_type(&self, id: EngineResourceTypeId) -> Option<&EngineHostResourceType> {
        self.hosts.resource(id)
    }

    #[must_use]
    pub fn host_symbol(&self, id: EngineHostSymbolId) -> Option<&EngineHostSymbol> {
        self.hosts.symbol(id)
    }

    pub fn grant_authority(&mut self, authority: Authority) {
        if !self.authorities.contains(&authority) {
            self.authorities.push(authority);
        }
    }

    #[must_use]
    pub fn with_authority(mut self, authority: Authority) -> Self {
        self.grant_authority(authority);
        self
    }

    #[must_use]
    pub fn with_authorities(mut self, authorities: impl IntoIterator<Item = Authority>) -> Self {
        for authority in authorities {
            self.grant_authority(authority);
        }
        self
    }

    pub fn set_task_execution(&mut self, mode: TaskExecutionMode) {
        self.task_execution = mode;
    }

    #[must_use]
    pub fn with_task_execution(mut self, mode: TaskExecutionMode) -> Self {
        self.set_task_execution(mode);
        self
    }

    #[must_use]
    pub const fn db(&self) -> &TuneDb {
        &self.db
    }

    pub const fn db_mut(&mut self) -> &mut TuneDb {
        &mut self.db
    }
}

pub(crate) fn has_error_diagnostics(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}
