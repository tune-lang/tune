use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf()
}

#[test]
fn std_examples_check_with_dyno() -> Result<(), String> {
    let dyno = env!("CARGO_BIN_EXE_dyno");
    let examples = [
        "examples/std/fs.tn",
        "examples/std/hash.tn",
        "examples/std/json.tn",
        "examples/std/math.tn",
        "examples/std/random.tn",
        "examples/std/time.tn",
    ];
    let root = repo_root();

    for example in examples {
        let output = Command::new(dyno)
            .arg("check")
            .arg(root.join(example))
            .current_dir(&root)
            .output()
            .map_err(|error| format!("failed to run dyno check for {example}: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "dyno check failed for {example}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    Ok(())
}
