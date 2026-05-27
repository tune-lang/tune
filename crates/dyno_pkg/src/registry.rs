use dyno_project::{PackageRef, VersionReq};

use crate::package::Package;

#[derive(Debug, Default, Clone)]
pub struct Registry {
    packages: Vec<Package>,
}

impl Registry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish(&mut self, package: Package) -> bool {
        if self
            .packages
            .iter()
            .any(|existing| existing.id == package.id && existing.version == package.version)
        {
            return false;
        }
        self.packages.push(package);
        true
    }

    #[must_use]
    pub fn resolve(&self, package: &PackageRef, requirement: &VersionReq) -> Option<&Package> {
        self.packages
            .iter()
            .find(|candidate| candidate.id == *package && candidate.version == *requirement)
    }

    #[must_use]
    pub fn packages(&self) -> &[Package] {
        &self.packages
    }
}
