use dyno_project::{Checksum, Dependency, ModuleRoot, PackageRef, VersionReq};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub id: PackageRef,
    pub version: VersionReq,
    pub checksum: Checksum,
    pub roots: Vec<ModuleRoot>,
    pub dependencies: Vec<Dependency>,
}

impl Package {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        checksum: impl Into<String>,
    ) -> Self {
        Self {
            id: PackageRef { name: name.into() },
            version: VersionReq(version.into()),
            checksum: Checksum(checksum.into()),
            roots: Vec::new(),
            dependencies: Vec::new(),
        }
    }
}
