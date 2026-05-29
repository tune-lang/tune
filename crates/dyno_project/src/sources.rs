use std::path::{Path, PathBuf};

use crate::manifest::{Manifest, ManifestParseError, ModuleRoot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSources {
    pub manifest_path: PathBuf,
    pub root: PathBuf,
    pub manifest: Manifest,
    pub sources: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectSourceLoadError {
    ReadManifest {
        path: PathBuf,
        message: String,
    },
    ParseManifest {
        path: PathBuf,
        error: ManifestParseError,
    },
    ReadSourceRoot {
        path: PathBuf,
        message: String,
    },
    ReadSource {
        path: PathBuf,
        message: String,
    },
    MissingEntry {
        path: PathBuf,
    },
}

pub fn load_project_dir(root: impl AsRef<Path>) -> Result<ProjectSources, ProjectSourceLoadError> {
    load_project_manifest(root.as_ref().join("dyno.toml"))
}

pub fn load_project_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<ProjectSources, ProjectSourceLoadError> {
    let manifest_path = manifest_path.as_ref().to_path_buf();
    let root = manifest_path
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    let manifest_text = std::fs::read_to_string(&manifest_path).map_err(|error| {
        ProjectSourceLoadError::ReadManifest {
            path: manifest_path.clone(),
            message: error.to_string(),
        }
    })?;
    let manifest = Manifest::from_toml(&manifest_text).map_err(|error| {
        ProjectSourceLoadError::ParseManifest {
            path: manifest_path.clone(),
            error,
        }
    })?;
    let mut sources = Vec::new();
    for module_root in &manifest.roots {
        let ModuleRoot::Source(source_root) = module_root else {
            continue;
        };
        collect_tune_sources(&root, &root.join(&source_root.0), &mut sources)?;
    }

    if !sources.iter().any(|(path, _)| path == &manifest.entry.0) {
        let entry = root.join(&manifest.entry.0);
        if !entry.exists() {
            return Err(ProjectSourceLoadError::MissingEntry { path: entry });
        }
        let text = std::fs::read_to_string(&entry).map_err(|error| {
            ProjectSourceLoadError::ReadSource {
                path: entry.clone(),
                message: error.to_string(),
            }
        })?;
        sources.push((manifest.entry.0.clone(), text));
    }

    Ok(ProjectSources {
        manifest_path,
        root,
        manifest,
        sources,
    })
}

fn collect_tune_sources(
    base: &Path,
    path: &Path,
    sources: &mut Vec<(String, String)>,
) -> Result<(), ProjectSourceLoadError> {
    let entries =
        std::fs::read_dir(path).map_err(|error| ProjectSourceLoadError::ReadSourceRoot {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
    for entry in entries {
        let entry = entry.map_err(|error| ProjectSourceLoadError::ReadSourceRoot {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_tune_sources(base, &path, sources)?;
            continue;
        }
        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("tn") {
            continue;
        }
        let text =
            std::fs::read_to_string(&path).map_err(|error| ProjectSourceLoadError::ReadSource {
                path: path.clone(),
                message: error.to_string(),
            })?;
        let project_path = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .trim_start_matches("./")
            .to_owned();
        sources.push((project_path, text));
    }
    Ok(())
}
