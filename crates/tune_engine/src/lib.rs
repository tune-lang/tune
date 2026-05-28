mod diagnostics;

pub use diagnostics::{
    diagnostic_from_result_error, diagnostic_from_result_error_with_sources,
    diagnostic_from_vm_fault, diagnostic_from_vm_fault_with_sources,
    diagnostics_from_runtime_value, diagnostics_from_runtime_value_with_sources,
};

use tune_db::{FileId, ModuleAnalysis, TuneDb};
use tune_diagnostics::Diagnostic;
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
        let module_plan = tune_plan::lower_resolved_module_to_plan(&check.module, &check.resolved);
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
        let ir = planned
            .iter()
            .copied()
            .map(tune_ir::lower_plan_function)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| EngineError::IrLower(format!("{error:?}")))?;
        let mut bytecode = tune_bytecode::lower_ir_functions(&ir)
            .map_err(|error| EngineError::BytecodeLower(format!("{error:?}")))?;
        bytecode.entry_function = Some(0);
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

fn reachable_functions(
    functions: &[tune_plan::PlanFunction],
    entry: &tune_plan::PlanFunction,
) -> Vec<usize> {
    let mut reachable = Vec::new();
    let mut pending = direct_call_targets(entry).collect::<Vec<_>>();
    pending.reverse();
    while let Some(target) = pending.pop() {
        let Some(index) = functions
            .iter()
            .position(|function| function_matches_target(function, target))
        else {
            continue;
        };
        if reachable.contains(&index) {
            continue;
        }
        reachable.push(index);
        for target in direct_call_targets(&functions[index]) {
            pending.push(target);
        }
    }
    reachable.sort_unstable();
    reachable
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FunctionTarget {
    Item(tune_hir::HirId),
    Member(tune_hir::MemberId),
}

fn function_matches_target(function: &tune_plan::PlanFunction, target: FunctionTarget) -> bool {
    match target {
        FunctionTarget::Item(item) => function.owner == Some(item) && function.member.is_none(),
        FunctionTarget::Member(member) => function.member == Some(member),
    }
}

fn direct_call_targets(
    function: &tune_plan::PlanFunction,
) -> impl Iterator<Item = FunctionTarget> + '_ {
    function.ops.iter().flat_map(direct_call_targets_in_op)
}

fn direct_call_targets_in_op(op: &tune_plan::PlanOp) -> Vec<FunctionTarget> {
    match op {
        tune_plan::PlanOp::DirectCall { target, .. } => Some(FunctionTarget::Item(*target)),
        tune_plan::PlanOp::MemberCall {
            member: Some(member),
            ..
        } => Some(FunctionTarget::Member(*member)),
        _ => None,
    }
    .into_iter()
    .chain(match op {
        tune_plan::PlanOp::If {
            branches, else_ops, ..
        } => branches
            .iter()
            .flat_map(|branch| {
                branch
                    .condition_ops
                    .iter()
                    .chain(branch.body_ops.iter())
                    .flat_map(direct_call_targets_in_op)
            })
            .chain(else_ops.iter().flat_map(direct_call_targets_in_op))
            .collect(),
        tune_plan::PlanOp::Match { arms, .. } => arms
            .iter()
            .flat_map(|arm| arm.body_ops.iter().flat_map(direct_call_targets_in_op))
            .collect(),
        _ => Vec::new(),
    })
    .collect()
}
