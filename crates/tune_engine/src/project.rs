use tune_runtime::Value;

use crate::executable::executable_from_compile;
use crate::{
    CheckReport, CompileReport, EngineError, ExecutableReport, ProjectEntry, ProjectHandle, Tune,
};

impl Tune {
    pub fn load_project(
        &mut self,
        manifest: dyno_project::manifest::Manifest,
    ) -> Result<ProjectHandle, EngineError> {
        let index = u32::try_from(self.projects.len()).map_err(|_| EngineError::AllocationLimit)?;
        self.projects.push(manifest);
        self.project_sources
            .push(crate::project_sources::ProjectSourceSet::default());
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
        let mut files = Vec::new();
        for (path, text) in sources {
            let file = self
                .add_source(path.clone(), text)
                .ok_or(EngineError::AllocationLimit)?;
            files.push(file);
            if path == entry_path {
                entry = Some(file);
            }
        }
        if let Some(source_set) = self.project_sources.get_mut(project.0 as usize) {
            *source_set = crate::project_sources::ProjectSourceSet::new(files);
        }
        let entry = entry.ok_or(EngineError::ProjectEntryNotFound(entry_path))?;
        Ok(ProjectEntry { project, entry })
    }

    pub fn load_project_manifest(
        &mut self,
        manifest_path: impl AsRef<std::path::Path>,
    ) -> Result<ProjectEntry, EngineError> {
        let loaded = dyno_project::load_project_manifest(manifest_path)
            .map_err(|error| EngineError::ProjectLoad(format!("{error:?}")))?;
        self.load_project_sources(loaded.manifest, loaded.sources)
    }

    pub fn load_project_dir(
        &mut self,
        root: impl AsRef<std::path::Path>,
    ) -> Result<ProjectEntry, EngineError> {
        let loaded = dyno_project::load_project_dir(root)
            .map_err(|error| EngineError::ProjectLoad(format!("{error:?}")))?;
        self.load_project_sources(loaded.manifest, loaded.sources)
    }

    pub fn check_project(
        &mut self,
        manifest_path: impl AsRef<std::path::Path>,
    ) -> Result<CheckReport, EngineError> {
        let entry = self.load_project_manifest(manifest_path)?;
        self.check_project_entry(entry)
    }

    pub fn compile_project(
        &mut self,
        manifest_path: impl AsRef<std::path::Path>,
    ) -> Result<CompileReport, EngineError> {
        let entry = self.load_project_manifest(manifest_path)?;
        self.compile_project_entry(entry)
    }

    pub fn executable_project(
        &mut self,
        manifest_path: impl AsRef<std::path::Path>,
    ) -> Result<ExecutableReport, EngineError> {
        let entry = self.load_project_manifest(manifest_path)?;
        self.executable_project_entry(entry)
    }

    pub fn run_project(
        &mut self,
        manifest_path: impl AsRef<std::path::Path>,
    ) -> Result<Value, EngineError> {
        let entry = self.load_project_manifest(manifest_path)?;
        self.run_project_entry(entry)
    }

    pub fn run_project_entry(&self, entry: ProjectEntry) -> Result<Value, EngineError> {
        self.project_entry_sources(entry)?;
        let executable = self.executable_project_entry(entry)?;
        self.runtime(executable).run_entry()
    }

    pub fn executable_project_entry(
        &self,
        entry: ProjectEntry,
    ) -> Result<ExecutableReport, EngineError> {
        self.project_entry_sources(entry)?;
        let compile = self.compile_project_entry(entry)?;
        executable_from_compile(compile)
    }

    pub fn compile_project_entry(&self, entry: ProjectEntry) -> Result<CompileReport, EngineError> {
        self.project_entry_sources(entry)?;
        let check = self.check_project_entry(entry)?;
        let module_plan =
            tune_plan::lower_analyzed_module_to_plan(&check.module, &check.resolved, &check.shape);

        Ok(CompileReport { check, module_plan })
    }

    pub fn check_project_entry(&self, entry: ProjectEntry) -> Result<CheckReport, EngineError> {
        let source_set = self.project_entry_sources(entry)?;
        let linked = crate::imports::link_entry_imports_for_files(
            &self.db,
            entry.entry,
            &self.hosts,
            &source_set.files,
            &source_set.import_aliases,
        )
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

    #[must_use]
    pub fn projects(&self) -> &[dyno_project::manifest::Manifest] {
        &self.projects
    }

    fn project_entry_sources(
        &self,
        entry: ProjectEntry,
    ) -> Result<&crate::project_sources::ProjectSourceSet, EngineError> {
        if self.projects.get(entry.project.0 as usize).is_none() {
            return Err(EngineError::NotImplemented("unknown project handle"));
        }
        self.project_sources
            .get(entry.project.0 as usize)
            .ok_or(EngineError::NotImplemented("unknown project handle"))
    }
}
