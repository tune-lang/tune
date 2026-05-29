use std::path::Path;

use tune_runtime::value::Value;

use crate::{CheckReport, CompileReport, EngineError, ExecutableReport, ProfileReport, Tune};

impl Tune {
    pub fn profile_frontend_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<ProfileReport, EngineError> {
        let file = self.add_path(path)?;
        self.profile_file_frontend(file)
    }

    pub fn check_path(&mut self, path: impl AsRef<Path>) -> Result<CheckReport, EngineError> {
        let file = self.add_path(path)?;
        self.check_file(file).ok_or(EngineError::FileNotFound(file))
    }

    pub fn compile_path(&mut self, path: impl AsRef<Path>) -> Result<CompileReport, EngineError> {
        let file = self.add_path(path)?;
        self.compile_file(file)
    }

    pub fn executable_path(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<ExecutableReport, EngineError> {
        let file = self.add_path(path)?;
        self.executable_file(file)
    }

    pub fn run_path(&mut self, path: impl AsRef<Path>) -> Result<Value, EngineError> {
        let file = self.add_path(path)?;
        self.run_file(file)
    }

    pub fn profile_path(&mut self, path: impl AsRef<Path>) -> Result<ProfileReport, EngineError> {
        let file = self.add_path(path)?;
        self.profile_file(file)
    }

    pub fn add_path(&mut self, path: impl AsRef<Path>) -> Result<tune_db::FileId, EngineError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|error| {
            EngineError::SourceLoad(format!("failed to read {}: {error}", path.display()))
        })?;
        self.add_file(path.to_string_lossy(), text)
            .ok_or(EngineError::AllocationLimit)
    }
}
