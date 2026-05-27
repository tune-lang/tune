use crate::lockfile::Lockfile;
use crate::manifest::{Manifest, ModuleRoot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResolution {
    pub roots: Vec<ModuleRoot>,
    pub locked_package_count: usize,
}

#[must_use]
pub fn resolve(manifest: &Manifest, lockfile: &Lockfile) -> ProjectResolution {
    let mut roots = manifest.roots.clone();
    roots.push(ModuleRoot::Std);
    ProjectResolution {
        roots,
        locked_package_count: lockfile.packages.len(),
    }
}
