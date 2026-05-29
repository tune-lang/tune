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

    pub fn from_toml(text: &str) -> Result<Self, ManifestParseError> {
        let mut section = "";
        let mut name = None;
        let mut edition = None;
        let mut entry = None;
        let mut host_profile = None;

        for raw_line in text.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(next_section) = line
                .strip_prefix('[')
                .and_then(|item| item.strip_suffix(']'))
            {
                section = next_section.trim();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                return Err(ManifestParseError::InvalidLine(line.to_owned()));
            };
            let key = key.trim();
            if matches!((section, key), ("project", "strict")) {
                continue;
            }
            let value = parse_string_value(value.trim())?;
            match (section, key) {
                ("project", "name") => name = Some(value),
                ("project", "edition") => edition = Some(value),
                ("project", "entry") => entry = Some(value),
                ("host", "profile") => host_profile = Some(value),
                ("dependencies", _) => {}
                _ => {}
            }
        }

        let name = name.ok_or(ManifestParseError::MissingProjectName)?;
        let entry = entry.ok_or(ManifestParseError::MissingEntry)?;
        let mut manifest = Self::new(name, entry);
        if let Some(edition) = edition {
            manifest.edition = Edition::from_manifest_value(&edition)?;
        }
        if let Some(profile) = host_profile {
            manifest.roots.push(ModuleRoot::Host(profile));
        }
        Ok(manifest)
    }

    #[must_use]
    pub fn to_toml(&self) -> String {
        let edition = self.edition.manifest_value();
        format!(
            r#"[project]
name = "{}"
edition = "{edition}"
entry = "{}"
strict = false

[dependencies]

[host]
profile = "dyno.default"
"#,
            self.name, self.entry.0
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestParseError {
    InvalidLine(String),
    InvalidString(String),
    MissingProjectName,
    MissingEntry,
    UnsupportedEdition(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    V1,
}

impl Edition {
    fn from_manifest_value(value: &str) -> Result<Self, ManifestParseError> {
        match value {
            "2026" | "v1" | "V1" => Ok(Self::V1),
            value => Err(ManifestParseError::UnsupportedEdition(value.to_owned())),
        }
    }

    const fn manifest_value(self) -> &'static str {
        match self {
            Self::V1 => "2026",
        }
    }
}

fn parse_string_value(value: &str) -> Result<String, ManifestParseError> {
    let Some(value) = value
        .strip_prefix('"')
        .and_then(|item| item.strip_suffix('"'))
    else {
        return Err(ManifestParseError::InvalidString(value.to_owned()));
    };
    Ok(value.to_owned())
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
