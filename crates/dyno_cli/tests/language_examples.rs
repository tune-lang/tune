use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf()
}

const LANGUAGE_EXAMPLES: &[&str] = &[
    "examples/language/01_values_and_flow.tn",
    "examples/language/02_functions_and_blocks.tn",
    "examples/language/03_structs_and_methods.tn",
    "examples/language/04_sequences_and_for.tn",
    "examples/language/05_enums_and_match.tn",
    "examples/language/06_result_propagation.tn",
    "examples/language/07_generics.tn",
    "examples/language/08_std_imports.tn",
    "examples/language/09_tasks.tn",
];

#[test]
fn language_examples_check_with_dyno() -> Result<(), String> {
    let dyno = env!("CARGO_BIN_EXE_dyno");
    let root = repo_root();

    for example in LANGUAGE_EXAMPLES {
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

#[test]
fn language_examples_are_formatter_stable() -> Result<(), String> {
    let dyno = env!("CARGO_BIN_EXE_dyno");
    let root = repo_root();

    for example in LANGUAGE_EXAMPLES {
        let output = Command::new(dyno)
            .arg("fmt")
            .arg("--check")
            .arg(root.join(example))
            .current_dir(&root)
            .output()
            .map_err(|error| format!("failed to run dyno fmt --check for {example}: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "dyno fmt --check failed for {example}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    Ok(())
}

#[test]
fn language_examples_run_with_dyno() -> Result<(), String> {
    let dyno = env!("CARGO_BIN_EXE_dyno");
    let root = repo_root();

    for example in LANGUAGE_EXAMPLES {
        let output = Command::new(dyno)
            .arg("run")
            .arg(root.join(example))
            .current_dir(&root)
            .output()
            .map_err(|error| format!("failed to run dyno run for {example}: {error}"))?;

        if !output.status.success() {
            return Err(format!(
                "dyno run failed for {example}\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        if output.stdout.is_empty() {
            return Err(format!("dyno run produced no output for {example}"));
        }
    }

    Ok(())
}
