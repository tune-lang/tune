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
}

pub struct CompileReport {
    pub check: CheckReport,
    pub functions: Vec<tune_plan::PlanFunction>,
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
        let functions = check
            .module
            .items
            .iter()
            .filter_map(|item| tune_plan::lower_resolved_item_to_plan(item, &check.resolved))
            .collect();

        Ok(CompileReport { check, functions })
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
        let _compiled = self.compile_file(file)?;
        Err(EngineError::NotImplemented(
            "typed bytecode lowering and VM execution",
        ))
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
    }
}
