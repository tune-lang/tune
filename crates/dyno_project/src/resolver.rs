use crate::lockfile::Lockfile;
use crate::manifest::{Dependency, Manifest, ModuleRoot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectResolution {
    pub roots: Vec<ModuleRoot>,
    pub locked_package_count: usize,
    pub missing_dependencies: Vec<Dependency>,
}

#[must_use]
pub fn resolve(manifest: &Manifest, lockfile: &Lockfile) -> ProjectResolution {
    let mut roots = manifest.roots.clone();
    let mut locked_package_count = 0;
    let mut missing_dependencies = Vec::new();
    for dependency in &manifest.dependencies {
        // v1 resolver skeleton uses exact lockfile requirements. Semver range satisfaction
        // belongs in the package solver, not in this project-root assembly step.
        if lockfile.packages.iter().any(|locked| {
            locked.package == dependency.package && locked.version == dependency.requirement
        }) {
            locked_package_count += 1;
            roots.push(ModuleRoot::Package(dependency.package.clone()));
        } else {
            missing_dependencies.push(dependency.clone());
        }
    }
    roots.push(ModuleRoot::Std);
    ProjectResolution {
        roots,
        locked_package_count,
        missing_dependencies,
    }
}
