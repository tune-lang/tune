mod report;

pub use report::{
    render_build_report, render_diagnostics_json, render_engine_error, render_engine_error_json,
    render_engine_error_with_sources, render_profile_report, render_runtime_boundary,
    render_runtime_boundary_with_sources,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliCommand {
    Build { path: Option<String> },
    Check { path: Option<String>, json: bool },
    Run { path: Option<String> },
    Profile { path: Option<String> },
    Fmt { path: Option<String>, check: bool },
    Explain { code: Option<String> },
    New { name: String },
    Lsp,
    Help,
}

pub fn parse_command(args: &[String]) -> Result<CliCommand, String> {
    match args {
        [] => Ok(CliCommand::Help),
        [flag] if flag == "-h" || flag == "--help" => Ok(CliCommand::Help),
        [command] if command == "build" => Ok(CliCommand::Build { path: None }),
        [command] if command == "run" => Ok(CliCommand::Run { path: None }),
        [command] if command == "check" => Ok(CliCommand::Check {
            path: None,
            json: false,
        }),
        [command] if command == "profile" => Ok(CliCommand::Profile { path: None }),
        [command] if command == "fmt" => Ok(CliCommand::Fmt {
            path: None,
            check: false,
        }),
        [command] if command == "explain" => Ok(CliCommand::Explain { code: None }),
        [command] if command == "lsp" => Ok(CliCommand::Lsp),
        [path] => Ok(CliCommand::Run {
            path: Some(path.clone()),
        }),
        [command, path] if command == "run" => Ok(CliCommand::Run {
            path: Some(path.clone()),
        }),
        [command, flag] if command == "check" && flag == "--json" => Ok(CliCommand::Check {
            path: None,
            json: true,
        }),
        [command, path] if command == "check" => Ok(CliCommand::Check {
            path: Some(path.clone()),
            json: false,
        }),
        [command, flag, path] if command == "check" && flag == "--json" => Ok(CliCommand::Check {
            path: Some(path.clone()),
            json: true,
        }),
        [command, path, flag] if command == "check" && flag == "--json" => Ok(CliCommand::Check {
            path: Some(path.clone()),
            json: true,
        }),
        [command, path] if command == "build" => Ok(CliCommand::Build {
            path: Some(path.clone()),
        }),
        [command, path] if command == "profile" => Ok(CliCommand::Profile {
            path: Some(path.clone()),
        }),
        [command, flag] if command == "fmt" && flag == "--check" => Ok(CliCommand::Fmt {
            path: None,
            check: true,
        }),
        [command, path] if command == "fmt" => Ok(CliCommand::Fmt {
            path: Some(path.clone()),
            check: false,
        }),
        [command, flag, path] if command == "fmt" && flag == "--check" => Ok(CliCommand::Fmt {
            path: Some(path.clone()),
            check: true,
        }),
        [command, path, flag] if command == "fmt" && flag == "--check" => Ok(CliCommand::Fmt {
            path: Some(path.clone()),
            check: true,
        }),
        [command, code] if command == "explain" => Ok(CliCommand::Explain {
            code: Some(code.clone()),
        }),
        [command, name] if command == "new" => Ok(CliCommand::New { name: name.clone() }),
        [command, ..] => Err(format!("unknown dyno command `{command}`")),
    }
}

#[must_use]
pub fn usage() -> &'static str {
    "usage: dyno new <name>\n       dyno check [--json] [file]\n       dyno run [file]\n       dyno build [file]\n       dyno profile [file]\n       dyno fmt [--check] [file]\n       dyno explain [code]\n       dyno lsp\n       dyno <file>"
}

#[must_use]
pub fn render_explain(code: Option<&str>) -> String {
    match code {
        Some(code) => tune_diagnostics::codes::explain(code).map_or_else(
            || format!("unknown diagnostic code `{code}`\n"),
            |info| {
                format!(
                    "{}: {}\n{}\n",
                    info.code.as_str(),
                    info.title,
                    info.explanation
                )
            },
        ),
        None => {
            tune_diagnostics::codes::all()
                .iter()
                .map(|info| format!("{}  {}", info.code.as_str(), info.title))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n"
        }
    }
}

pub fn format_file(path: impl AsRef<std::path::Path>) -> Result<bool, String> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let formatted = tune_fmt::format_source(&source);
    if formatted == source {
        return Ok(false);
    }
    std::fs::write(path, formatted)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(true)
}

pub fn file_needs_format(path: impl AsRef<std::path::Path>) -> Result<bool, String> {
    let path = path.as_ref();
    let source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok(tune_fmt::format_source(&source) != source)
}

pub fn format_project(
    root: impl AsRef<std::path::Path>,
) -> Result<Vec<std::path::PathBuf>, String> {
    let loaded = dyno_project::load_project_dir(&root).map_err(|error| format!("{error:?}"))?;
    let mut changed = Vec::new();
    for (path, _) in loaded.sources {
        let path = loaded.root.join(path);
        if format_file(&path)? {
            changed.push(path);
        }
    }
    Ok(changed)
}

pub fn check_format_project(
    root: impl AsRef<std::path::Path>,
) -> Result<Vec<std::path::PathBuf>, String> {
    let loaded = dyno_project::load_project_dir(&root).map_err(|error| format!("{error:?}"))?;
    let mut unformatted = Vec::new();
    for (path, _) in loaded.sources {
        let path = loaded.root.join(path);
        if file_needs_format(&path)? {
            unformatted.push(path);
        }
    }
    Ok(unformatted)
}

#[must_use]
pub fn default_tune() -> tune_engine::Tune {
    tune_engine::Tune::new()
        .with_std()
        .with_authority(tune_host::Authority("io.write".into()))
        .with_authority(tune_host::Authority("io.error".into()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProject {
    pub name: String,
    pub root: std::path::PathBuf,
    pub manifest: std::path::PathBuf,
    pub entry: std::path::PathBuf,
}

pub fn create_project(name: &str) -> Result<NewProject, String> {
    create_project_in(".", name)
}

pub fn create_project_in(
    base: impl AsRef<std::path::Path>,
    name: &str,
) -> Result<NewProject, String> {
    validate_project_name(name)?;
    let root = base.as_ref().join(name);
    if root.exists() {
        return Err(format!("project path `{name}` already exists"));
    }
    let src = root.join("src");
    std::fs::create_dir_all(&src).map_err(|error| format!("failed to create {src:?}: {error}"))?;

    let manifest = root.join("dyno.toml");
    let entry = src.join("main.tn");
    let project_manifest = dyno_project::Manifest::new(name, "src/main.tn");
    std::fs::write(&manifest, project_manifest.to_toml())
        .map_err(|error| format!("failed to write {manifest:?}: {error}"))?;
    std::fs::write(&entry, entry_template())
        .map_err(|error| format!("failed to write {entry:?}: {error}"))?;

    Ok(NewProject {
        name: name.to_owned(),
        root,
        manifest,
        entry,
    })
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("project name must not be empty".to_owned());
    }
    if name == "." || name == ".." || name.contains(std::path::MAIN_SEPARATOR) {
        return Err(format!("invalid project name `{name}`"));
    }
    if !name
        .chars()
        .all(|item| item.is_ascii_alphanumeric() || item == '_' || item == '-')
    {
        return Err(format!(
            "project name `{name}` may only contain ASCII letters, numbers, `_`, and `-`"
        ));
    }
    Ok(())
}

fn entry_template() -> &'static str {
    "let message: String = \"hello from Dyno\"\n"
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedProject {
    pub manifest: dyno_project::Manifest,
    pub sources: Vec<(String, String)>,
}

pub fn load_project_from_dir(root: impl AsRef<std::path::Path>) -> Result<LoadedProject, String> {
    let loaded = dyno_project::load_project_dir(root).map_err(|error| format!("{error:?}"))?;
    Ok(LoadedProject {
        manifest: loaded.manifest,
        sources: loaded.sources,
    })
}
