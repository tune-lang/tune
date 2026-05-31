fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = match dyno_cli::parse_command(&args) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", dyno_cli::usage());
            std::process::exit(2);
        }
    };
    if let dyno_cli::CliCommand::New { name } = command {
        match dyno_cli::create_project(&name) {
            Ok(project) => {
                println!("created {}", project.root.display());
                println!("  {}", project.manifest.display());
                println!("  {}", project.entry.display());
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if matches!(command, dyno_cli::CliCommand::Lsp) {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut reader = std::io::BufReader::new(stdin.lock());
        let mut writer = stdout.lock();
        let mut server = tune_lsp::JsonRpcServer::new();
        if let Err(error) = tune_lsp::run_stdio(&mut reader, &mut writer, &mut server) {
            eprintln!("dyno lsp failed: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let dyno_cli::CliCommand::Fmt { ref path, check } = command {
        match (path, check) {
            (Some(path), true) => match dyno_cli::file_needs_format(path) {
                Ok(true) => {
                    println!("would format {path}");
                    std::process::exit(1);
                }
                Ok(false) => println!("already formatted {path}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            },
            (Some(path), false) => match dyno_cli::format_file(path) {
                Ok(true) => println!("formatted {path}"),
                Ok(false) => println!("already formatted {path}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            },
            (None, true) => match dyno_cli::check_format_project(".") {
                Ok(unformatted) => {
                    if unformatted.is_empty() {
                        println!("all files formatted");
                    } else {
                        for path in unformatted {
                            println!("would format {}", path.display());
                        }
                        std::process::exit(1);
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            },
            (None, false) => match dyno_cli::format_project(".") {
                Ok(changed) => {
                    for path in changed {
                        println!("formatted {}", path.display());
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            },
        }
        return;
    }
    if let dyno_cli::CliCommand::Explain { ref code } = command {
        print!("{}", dyno_cli::render_explain(code.as_deref()));
        if code
            .as_deref()
            .is_some_and(|code| tune_diagnostics::codes::explain(code).is_none())
        {
            std::process::exit(1);
        }
        return;
    }

    let path = match command {
        dyno_cli::CliCommand::Build { ref path }
        | dyno_cli::CliCommand::Check { ref path, .. }
        | dyno_cli::CliCommand::Fmt { ref path, .. }
        | dyno_cli::CliCommand::Profile { ref path }
        | dyno_cli::CliCommand::Run { ref path } => path.as_ref(),
        dyno_cli::CliCommand::Explain { .. } => unreachable!(),
        dyno_cli::CliCommand::New { .. } => unreachable!(),
        dyno_cli::CliCommand::Lsp => unreachable!(),
        dyno_cli::CliCommand::Help => {
            println!("{}", dyno_cli::usage());
            return;
        }
    };
    let Some(path) = path else {
        run_project_command_at(command, "dyno.toml");
        return;
    };
    if std::path::Path::new(path).is_dir() {
        let manifest = std::path::Path::new(path).join("dyno.toml");
        if manifest.exists() {
            run_project_command_at(command, &manifest.to_string_lossy());
        } else {
            eprintln!("error: `{path}` is a directory with no dyno.toml");
            std::process::exit(1);
        }
        return;
    }
    let mut tune = dyno_cli::default_tune();

    if matches!(command, dyno_cli::CliCommand::Profile { .. }) {
        match tune.profile_file(path) {
            Ok(report) => {
                print!("{}", dyno_cli::render_profile_report(&report));
                if !report.diagnostics.is_empty() {
                    for diagnostic in &report.diagnostics {
                        eprintln!(
                            "{}",
                            tune_diagnostics::render::render_plain_with_sources(
                                diagnostic,
                                tune.db()
                            )
                        );
                    }
                    std::process::exit(1);
                }
                if report.stop_reason.is_some() {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db()) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        return;
    }

    if matches!(command, dyno_cli::CliCommand::Build { .. }) {
        match tune.executable_file(path) {
            Ok(report) => println!("{}", dyno_cli::render_build_report(&report)),
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db()) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        return;
    }

    if let dyno_cli::CliCommand::Check { json, .. } = command {
        match tune.check_file(path) {
            Ok(report) => {
                if json {
                    print!("{}", dyno_cli::render_diagnostics_json(&report.diagnostics));
                } else {
                    for diagnostic in &report.diagnostics {
                        eprintln!(
                            "{}",
                            tune_diagnostics::render::render_plain_with_sources(
                                diagnostic,
                                tune.db()
                            )
                        );
                    }
                }
                if report.diagnostics.is_empty() {
                    return;
                }
            }
            Err(error) => {
                if json {
                    print!("{}", dyno_cli::render_engine_error_json(&error));
                } else {
                    for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db())
                    {
                        eprintln!("{diagnostic}");
                    }
                }
            }
        }
        std::process::exit(1);
    }

    match tune.run_file(path) {
        Ok(value) => {
            handle_run_value(value, tune.db());
        }
        Err(error) => {
            for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db()) {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
    }
}

fn run_project_command_at(command: dyno_cli::CliCommand, manifest: &str) {
    let mut tune = dyno_cli::default_tune();
    let entry = match tune.load_project_manifest(manifest) {
        Ok(entry) => entry,
        Err(error) => {
            for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db()) {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
    };

    if let dyno_cli::CliCommand::Check { json, .. } = command {
        let report = match tune.check_project_entry(entry) {
            Ok(report) => report,
            Err(error) => {
                if json {
                    print!("{}", dyno_cli::render_engine_error_json(&error));
                } else {
                    for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db())
                    {
                        eprintln!("{diagnostic}");
                    }
                }
                std::process::exit(1);
            }
        };
        if json {
            print!("{}", dyno_cli::render_diagnostics_json(&report.diagnostics));
        } else {
            for diagnostic in &report.diagnostics {
                eprintln!(
                    "{}",
                    tune_diagnostics::render::render_plain_with_sources(diagnostic, tune.db())
                );
            }
        }
        if !report.diagnostics.is_empty() {
            std::process::exit(1);
        }
        return;
    }

    if matches!(command, dyno_cli::CliCommand::Build { .. }) {
        match tune.executable_project_entry(entry) {
            Ok(report) => println!("{}", dyno_cli::render_build_report(&report)),
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db()) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        return;
    }

    if matches!(command, dyno_cli::CliCommand::Profile { .. }) {
        match tune.profile_project_entry(entry) {
            Ok(report) => {
                print!("{}", dyno_cli::render_profile_report(&report));
                if !report.diagnostics.is_empty() || report.stop_reason.is_some() {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db()) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        return;
    }

    match tune.run_project_entry(entry) {
        Ok(value) => {
            handle_run_value(value, tune.db());
        }
        Err(error) => {
            for diagnostic in dyno_cli::render_engine_error_with_sources(&error, tune.db()) {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
    }
}

fn handle_run_value(value: tune_runtime::Value, db: &tune_db::TuneDb) {
    let diagnostics = dyno_cli::render_runtime_boundary_with_sources(&value, db);
    if diagnostics.is_empty() {
        return;
    }
    for diagnostic in diagnostics {
        eprintln!("{diagnostic}");
    }
    std::process::exit(1);
}
