use tune_db::FileId;

use crate::{EngineError, ProjectEntry, Tune};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectPackageSources {
    pub package: dyno_project::PackageRef,
    pub sources: Vec<(String, String)>,
}

impl ProjectPackageSources {
    #[must_use]
    pub fn new(
        package: dyno_project::PackageRef,
        sources: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        Self {
            package,
            sources: sources.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProjectSourceSet {
    pub(crate) files: Vec<FileId>,
    pub(crate) import_aliases: Vec<(String, FileId)>,
}

impl ProjectSourceSet {
    pub(crate) fn new(files: Vec<FileId>) -> Self {
        Self {
            files,
            import_aliases: Vec::new(),
        }
    }
}

impl Tune {
    pub fn load_project_sources_with_packages(
        &mut self,
        manifest: dyno_project::Manifest,
        lockfile: &dyno_project::Lockfile,
        sources: impl IntoIterator<Item = (String, String)>,
        packages: impl IntoIterator<Item = ProjectPackageSources>,
    ) -> Result<ProjectEntry, EngineError> {
        let resolution = dyno_project::resolve(&manifest, lockfile);
        if !resolution.missing_dependencies.is_empty() {
            return Err(EngineError::ProjectLoad(
                "project has dependencies missing from dyno.lock".into(),
            ));
        }

        let entry_path = manifest.entry.0.clone();
        let project = self.load_project(manifest)?;
        let mut entry = None;
        let mut source_set = ProjectSourceSet::default();
        for (path, text) in sources {
            let file = self
                .add_file(path.clone(), text)
                .ok_or(EngineError::AllocationLimit)?;
            source_set.files.push(file);
            if path == entry_path {
                entry = Some(file);
            }
        }

        let package_roots = resolution
            .roots
            .iter()
            .filter_map(|root| match root {
                dyno_project::ModuleRoot::Package(package) => Some(package),
                _ => None,
            })
            .collect::<Vec<_>>();
        for package in packages {
            if !package_roots.contains(&&package.package) {
                return Err(EngineError::ProjectLoad(format!(
                    "package `{}` is not a locked dependency",
                    package.package.name
                )));
            }
            self.add_package_sources(&mut source_set, package)?;
        }

        if let Some(project_sources) = self.project_sources.get_mut(project.0 as usize) {
            *project_sources = source_set;
        }
        let entry = entry.ok_or(EngineError::ProjectEntryNotFound(entry_path))?;
        Ok(ProjectEntry { project, entry })
    }

    fn add_package_sources(
        &mut self,
        source_set: &mut ProjectSourceSet,
        package: ProjectPackageSources,
    ) -> Result<(), EngineError> {
        for (path, text) in package.sources {
            let internal_path = format!("packages/{}/{}", package.package.name, path);
            let file = self
                .add_file(internal_path, text)
                .ok_or(EngineError::AllocationLimit)?;
            source_set.files.push(file);
            for alias in package_import_aliases(&package.package, &path) {
                source_set.import_aliases.push((alias, file));
            }
        }
        Ok(())
    }
}

fn package_import_aliases(package: &dyno_project::PackageRef, path: &str) -> Vec<String> {
    let normalized = path.trim_start_matches("./");
    let module_path = normalized.strip_prefix("src/").unwrap_or(normalized);
    let module = module_path.strip_suffix(".tn").unwrap_or(module_path);
    if module == "lib" {
        return vec![package.name.clone()];
    }
    vec![
        format!("{}/{}", package.name, normalized),
        format!("{}/{}", package.name, module),
    ]
}
