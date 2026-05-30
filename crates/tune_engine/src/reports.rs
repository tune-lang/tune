use tune_db::FileId;
use tune_diagnostics::Diagnostic;

pub type SourceId = FileId;

pub struct CheckReport {
    pub file: SourceId,
    pub diagnostics: Vec<Diagnostic>,
    pub module: tune_hir::module::Module,
    pub resolved: tune_resolve::ResolvedModule,
    pub shape: Vec<tune_shape::ShapeAnalysis>,
}

pub struct CompileReport {
    pub check: CheckReport,
    pub module_plan: tune_plan::PlanModule,
}

pub struct ExecutableReport {
    pub compile: CompileReport,
    pub ir: Vec<tune_ir::IrFunction>,
    pub bytecode: tune_bytecode::artifact::BytecodeArtifact,
    pub host_value_types: Vec<tune_vm::VmHostValueType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryPoint {
    Source(SourceId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProjectHandle(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectEntry {
    pub project: ProjectHandle,
    pub entry: SourceId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    FileNotFound(SourceId),
    AllocationLimit,
    Diagnostics(Vec<Diagnostic>),
    IrLower(String),
    BytecodeLower(String),
    MissingEntry,
    ProjectEntryNotFound(String),
    ProjectLoad(String),
    SourceLoad(String),
    NotImplemented(&'static str),
}
