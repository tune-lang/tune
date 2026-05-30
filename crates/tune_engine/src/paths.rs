use std::path::Path;

use tune_runtime::value::Value;

use crate::{CheckReport, CompileReport, EngineError, ExecutableReport, ProfileReport, Tune};

impl Tune {
    pub fn profile_file_frontend(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<ProfileReport, EngineError> {
        let file = self.load_file(path)?;
        self.profile_source_frontend(file)
    }

    pub fn check_file(&mut self, path: impl AsRef<Path>) -> Result<CheckReport, EngineError> {
        let file = self.load_file(path)?;
        self.check_source(file)
            .ok_or(EngineError::FileNotFound(file))
    }

    pub fn compile_file(&mut self, path: impl AsRef<Path>) -> Result<CompileReport, EngineError> {
        let file = self.load_file(path)?;
        self.compile_source(file)
    }

    pub fn executable_file(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<ExecutableReport, EngineError> {
        let file = self.load_file(path)?;
        self.executable_source(file)
    }

    pub fn run_file(&mut self, path: impl AsRef<Path>) -> Result<Value, EngineError> {
        let file = self.load_file(path)?;
        self.run_source(file)
    }

    pub fn profile_file(&mut self, path: impl AsRef<Path>) -> Result<ProfileReport, EngineError> {
        let file = self.load_file(path)?;
        self.profile_source(file)
    }

    pub fn load_file(&mut self, path: impl AsRef<Path>) -> Result<tune_db::FileId, EngineError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|error| {
            EngineError::SourceLoad(format!("failed to read {}: {error}", path.display()))
        })?;
        self.add_source(path.to_string_lossy(), text)
            .ok_or(EngineError::AllocationLimit)
    }
}
