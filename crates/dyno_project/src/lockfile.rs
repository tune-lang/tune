use crate::manifest::{PackageRef, VersionReq};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lockfile {
    pub packages: Vec<LockedPackage>,
}

impl Lockfile {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, package: LockedPackage) -> bool {
        if self
            .packages
            .iter()
            .any(|locked| locked.package == package.package)
        {
            return false;
        }
        self.packages.push(package);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockedPackage {
    pub package: PackageRef,
    pub version: VersionReq,
    pub checksum: Checksum,
    pub source: PackageSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageSource {
    Registry(String),
    Path(String),
}
