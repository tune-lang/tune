use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf()
}

#[test]
fn every_registered_diagnostic_has_public_docs() -> Result<(), String> {
    let root = repo_root();

    for info in tune_diagnostics::codes::all() {
        let path = root
            .join("docs")
            .join("diagnostics")
            .join(format!("{}.md", info.code.as_str()));
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

        if !text.contains(info.code.as_str()) {
            return Err(format!("{} does not mention its code", path.display()));
        }
        if !text
            .to_ascii_lowercase()
            .contains(&info.title.to_ascii_lowercase())
        {
            return Err(format!(
                "{} does not mention diagnostic title `{}`",
                path.display(),
                info.title
            ));
        }
    }

    Ok(())
}
