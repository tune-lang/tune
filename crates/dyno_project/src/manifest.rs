#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub edition: Edition,
    pub entry: ModulePath,
    pub roots: Vec<ModuleRoot>,
    pub profile: Profile,
    pub dependencies: Vec<Dependency>,
}

impl Manifest {
    #[must_use]
    pub fn new(name: impl Into<String>, entry: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            edition: Edition::V1,
            entry: ModulePath(entry.into()),
            roots: vec![ModuleRoot::Source(ModulePath("src".into()))],
            profile: Profile::Debug,
            dependencies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    V1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Profile {
    Debug,
    Release,
    Host(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleRoot {
    Source(ModulePath),
    Std,
    Host(String),
    Package(PackageRef),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageRef {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub package: PackageRef,
    pub requirement: VersionReq,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq(pub String);
