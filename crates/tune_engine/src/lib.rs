mod diagnostics;
mod host;
mod imports;
mod imports_remap;
mod reachable;

use diagnostics::{diagnostic_from_bytecode_lower_error, diagnostic_from_ir_lower_error};
pub use diagnostics::{
    diagnostic_from_result_error, diagnostic_from_result_error_with_sources,
    diagnostic_from_vm_fault, diagnostic_from_vm_fault_with_sources,
    diagnostics_from_runtime_value, diagnostics_from_runtime_value_with_sources,
};
pub use host::{EngineHostSymbol, EngineHostSymbolId, HostRegistration};

use tune_db::{FileId, ModuleAnalysis, TuneDb};
use tune_diagnostics::Diagnostic;
use tune_host::module::HostModule;
use tune_runtime::value::Value;

use crate::reachable::reachable_functions;

#[derive(Default)]
pub struct Tune {
    db: TuneDb,
    hosts: host::HostRegistry,
    projects: Vec<dyno_project::manifest::Manifest>,
}

pub struct CheckReport {
    pub file: FileId,
    pub diagnostics: Vec<Diagnostic>,
    pub module: tune_hir::module::Module,
    pub resolved: tune_resolve::ResolvedModule,
    pub shape: Vec<tune_shape::ShapeAnalysis>,
}

pub struct CompileReport {
    pub check: CheckReport,
    pub module_plan: tune_plan::PlanModule,
    pub functions: Vec<tune_plan::PlanFunction>,
}

pub struct ExecutableReport {
    pub compile: CompileReport,
    pub ir: Vec<tune_ir::IrFunction>,
    pub bytecode: tune_bytecode::artifact::BytecodeArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPoint {
    File(FileId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectHandle(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectEntry {
    pub project: ProjectHandle,
    pub entry: FileId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    FileNotFound(FileId),
    AllocationLimit,
    Diagnostics(Vec<Diagnostic>),
    IrLower(String),
    BytecodeLower(String),
    MissingEntry,
    ProjectEntryNotFound(String),
    NotImplemented(&'static str),
}

impl Tune {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&mut self, path: impl Into<String>, text: impl Into<String>) -> Option<FileId> {
        self.db.add_file(path, text)
    }

    #[must_use]
    pub fn check_file(&self, file: FileId) -> Option<CheckReport> {
        self.db
            .analyze_file(file)
            .map(|analysis| report_from_analysis(file, analysis))
    }

    pub fn check_source(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Option<CheckReport> {
        let file = self.add_file(path, text)?;
        self.check_file(file)
    }

    pub fn compile_file(&self, file: FileId) -> Result<CompileReport, EngineError> {
        let check = self
            .check_file(file)
            .ok_or(EngineError::FileNotFound(file))?;
        let module_plan =
            tune_plan::lower_analyzed_module_to_plan(&check.module, &check.resolved, &check.shape);
        let functions = module_plan.functions.clone();

        Ok(CompileReport {
            check,
            module_plan,
            functions,
        })
    }

    pub fn compile_source(
        &mut self,
        path: impl Into<String>,
        text: impl Into<String>,
    ) -> Result<CompileReport, EngineError> {
        let file = self
            .add_file(path, text)
            .ok_or(EngineError::AllocationLimit)?;
        self.compile_file(file)
    }

    pub fn run_file(&self, file: FileId) -> Result<Value, EngineError> {
        self.run_entry(EntryPoint::File(file))
    }

    pub fn run_entry(&self, entry: EntryPoint) -> Result<Value, EngineError> {
        let executable = self.executable_entry(entry)?;
        let mut vm = tune_vm::Vm::new(executable.bytecode);
        vm.run_entry().map_err(|fault| {
            EngineError::Diagnostics(vec![diagnostic_from_vm_fault_with_sources(
                &fault, &self.db,
            )])
        })
    }

    pub fn executable_file(&self, file: FileId) -> Result<ExecutableReport, EngineError> {
        self.executable_entry(EntryPoint::File(file))
    }

    pub fn executable_entry(&self, entry: EntryPoint) -> Result<ExecutableReport, EngineError> {
        let EntryPoint::File(file) = entry;
        let compile = self.compile_file(file)?;
        executable_from_compile(compile)
    }

    pub fn load_project(
        &mut self,
        manifest: dyno_project::manifest::Manifest,
    ) -> Result<ProjectHandle, EngineError> {
        let index = u32::try_from(self.projects.len()).map_err(|_| EngineError::AllocationLimit)?;
        self.projects.push(manifest);
        Ok(ProjectHandle(index))
    }

    pub fn resolve_project(
        &self,
        project: ProjectHandle,
        lockfile: &dyno_project::lockfile::Lockfile,
    ) -> Result<dyno_project::ProjectResolution, EngineError> {
        let manifest = self
            .projects
            .get(project.0 as usize)
            .ok_or(EngineError::NotImplemented("unknown project handle"))?;
        Ok(dyno_project::resolve(manifest, lockfile))
    }

    pub fn load_project_sources(
        &mut self,
        manifest: dyno_project::manifest::Manifest,
        sources: impl IntoIterator<Item = (String, String)>,
    ) -> Result<ProjectEntry, EngineError> {
        let entry_path = manifest.entry.0.clone();
        let project = self.load_project(manifest)?;
        let mut entry = None;
        for (path, text) in sources {
            let file = self
                .add_file(path.clone(), text)
                .ok_or(EngineError::AllocationLimit)?;
            if path == entry_path {
                entry = Some(file);
            }
        }
        let entry = entry.ok_or(EngineError::ProjectEntryNotFound(entry_path))?;
        Ok(ProjectEntry { project, entry })
    }

    pub fn run_project_entry(&self, entry: ProjectEntry) -> Result<Value, EngineError> {
        if self.projects.get(entry.project.0 as usize).is_none() {
            return Err(EngineError::NotImplemented("unknown project handle"));
        }
        let executable = self.executable_project_entry(entry)?;
        let mut vm = tune_vm::Vm::new(executable.bytecode);
        vm.run_entry().map_err(|fault| {
            EngineError::Diagnostics(vec![diagnostic_from_vm_fault_with_sources(
                &fault, &self.db,
            )])
        })
    }

    pub fn executable_project_entry(
        &self,
        entry: ProjectEntry,
    ) -> Result<ExecutableReport, EngineError> {
        if self.projects.get(entry.project.0 as usize).is_none() {
            return Err(EngineError::NotImplemented("unknown project handle"));
        }
        let compile = self.compile_project_entry(entry)?;
        executable_from_compile(compile)
    }

    pub fn compile_project_entry(&self, entry: ProjectEntry) -> Result<CompileReport, EngineError> {
        if self.projects.get(entry.project.0 as usize).is_none() {
            return Err(EngineError::NotImplemented("unknown project handle"));
        }
        let check = self.check_project_entry(entry)?;
        let module_plan =
            tune_plan::lower_analyzed_module_to_plan(&check.module, &check.resolved, &check.shape);
        let functions = module_plan.functions.clone();

        Ok(CompileReport {
            check,
            module_plan,
            functions,
        })
    }

    pub fn check_project_entry(&self, entry: ProjectEntry) -> Result<CheckReport, EngineError> {
        if self.projects.get(entry.project.0 as usize).is_none() {
            return Err(EngineError::NotImplemented("unknown project handle"));
        }
        let linked = imports::link_entry_imports(&self.db, entry.entry)
            .ok_or(EngineError::FileNotFound(entry.entry))?;
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

        Ok(CheckReport {
            file: entry.entry,
            diagnostics,
            module: linked.module,
            resolved,
            shape,
        })
    }

    pub fn register_host(&mut self, host: &impl tune_host::Host) -> HostRegistration {
        self.hosts.register(host)
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
    pub fn host_symbol(&self, id: EngineHostSymbolId) -> Option<&EngineHostSymbol> {
        self.hosts.symbol(id)
    }

    #[must_use]
    pub fn projects(&self) -> &[dyno_project::manifest::Manifest] {
        &self.projects
    }

    #[must_use]
    pub const fn db(&self) -> &TuneDb {
        &self.db
    }

    pub const fn db_mut(&mut self) -> &mut TuneDb {
        &mut self.db
    }
}

fn report_from_analysis(file: FileId, analysis: ModuleAnalysis) -> CheckReport {
    CheckReport {
        file,
        diagnostics: analysis.diagnostics(),
        module: analysis.module,
        resolved: analysis.resolved,
        shape: analysis.shape,
    }
}

fn executable_from_compile(compile: CompileReport) -> Result<ExecutableReport, EngineError> {
    if !compile.check.diagnostics.is_empty() {
        return Err(EngineError::Diagnostics(compile.check.diagnostics.clone()));
    }
    let entry_plan = compile
        .module_plan
        .entry
        .as_ref()
        .ok_or(EngineError::MissingEntry)?;
    let reachable = reachable_functions(&compile.functions, entry_plan);
    let planned = core::iter::once(entry_plan)
        .chain(reachable.iter().map(|index| &compile.functions[*index]))
        .collect::<Vec<_>>();
    let mut ir = Vec::new();
    for plan in planned {
        let function = tune_ir::lower_plan_function(plan).map_err(|error| {
            EngineError::Diagnostics(vec![diagnostic_from_ir_lower_error(
                &plan.name, plan.span, &error,
            )])
        })?;
        ir.push(function);
    }
    let mut bytecode = tune_bytecode::lower_ir_functions(&ir).map_err(|error| {
        EngineError::Diagnostics(vec![diagnostic_from_bytecode_lower_error(&error)])
    })?;
    bytecode.entry_function = Some(0);
    Ok(ExecutableReport {
        compile,
        ir,
        bytecode,
    })
}
