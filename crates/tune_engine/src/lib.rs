use tune_db::{FileId, ModuleAnalysis, TuneDb};
use tune_diagnostics::Diagnostic;
use tune_hir::item::ItemKind;
use tune_host::module::HostModule;
use tune_runtime::value::Value;

#[derive(Default)]
pub struct Tune {
    db: TuneDb,
    host_modules: Vec<HostModule>,
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
    pub functions: Vec<tune_plan::PlanFunction>,
    pub entry_function: Option<usize>,
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
pub struct HostRegistration {
    pub module_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    FileNotFound(FileId),
    AllocationLimit,
    Diagnostics(Vec<Diagnostic>),
    IrLower(String),
    BytecodeLower(String),
    Vm(String),
    MissingEntry,
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
        let mut functions = Vec::new();
        let mut entry_function = None;
        for item in &check.module.items {
            let Some(plan) =
                tune_plan::lower_resolved_module_item_to_plan(&check.module, item, &check.resolved)
            else {
                continue;
            };
            if entry_function.is_none() && item.kind == ItemKind::Let {
                entry_function = Some(functions.len());
            }
            functions.push(plan);
        }

        Ok(CompileReport {
            check,
            functions,
            entry_function,
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
        vm.run_entry()
            .map_err(|error| EngineError::Vm(format!("{error:?}")))
    }

    pub fn executable_file(&self, file: FileId) -> Result<ExecutableReport, EngineError> {
        self.executable_entry(EntryPoint::File(file))
    }

    pub fn executable_entry(&self, entry: EntryPoint) -> Result<ExecutableReport, EngineError> {
        let EntryPoint::File(file) = entry;
        let compile = self.compile_file(file)?;
        if !compile.check.diagnostics.is_empty() {
            return Err(EngineError::Diagnostics(compile.check.diagnostics.clone()));
        }
        let entry_function = compile.entry_function.ok_or(EngineError::MissingEntry)?;
        let reachable = reachable_functions(&compile.functions, entry_function);
        let bytecode_entry = reachable
            .iter()
            .position(|index| *index == entry_function)
            .ok_or(EngineError::MissingEntry)?;
        let planned = reachable
            .iter()
            .map(|index| &compile.functions[*index])
            .collect::<Vec<_>>();
        let ir = planned
            .iter()
            .copied()
            .map(tune_ir::lower_plan_function)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| EngineError::IrLower(format!("{error:?}")))?;
        let mut bytecode = tune_bytecode::lower_ir_functions(&ir)
            .map_err(|error| EngineError::BytecodeLower(format!("{error:?}")))?;
        bytecode.entry_function =
            Some(u32::try_from(bytecode_entry).map_err(|_| EngineError::AllocationLimit)?);
        Ok(ExecutableReport {
            compile,
            ir,
            bytecode,
        })
    }

    pub fn load_project(
        &mut self,
        manifest: dyno_project::manifest::Manifest,
    ) -> Result<ProjectHandle, EngineError> {
        let index = u32::try_from(self.projects.len()).map_err(|_| EngineError::AllocationLimit)?;
        self.projects.push(manifest);
        Ok(ProjectHandle(index))
    }

    pub fn register_host(&mut self, host: &impl tune_host::Host) -> HostRegistration {
        let modules = host.modules();
        let module_count = modules.len();
        self.host_modules.extend(modules);
        HostRegistration { module_count }
    }

    #[must_use]
    pub fn host_modules(&self) -> &[HostModule] {
        &self.host_modules
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

fn reachable_functions(functions: &[tune_plan::PlanFunction], entry: usize) -> Vec<usize> {
    let mut reachable = Vec::new();
    let mut pending = vec![entry];
    while let Some(index) = pending.pop() {
        if reachable.contains(&index) {
            continue;
        }
        reachable.push(index);
        for target in direct_call_targets(&functions[index]) {
            if let Some(target_index) = functions
                .iter()
                .position(|function| function.owner == Some(target))
            {
                pending.push(target_index);
            }
        }
    }
    reachable.sort_unstable();
    reachable
}

fn direct_call_targets(
    function: &tune_plan::PlanFunction,
) -> impl Iterator<Item = tune_hir::HirId> + '_ {
    function.ops.iter().filter_map(|op| match op {
        tune_plan::PlanOp::DirectCall { target } => Some(*target),
        _ => None,
    })
}
