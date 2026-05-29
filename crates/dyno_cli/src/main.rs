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

    let path = match command {
        dyno_cli::CliCommand::Build { ref path }
        | dyno_cli::CliCommand::Check { ref path }
        | dyno_cli::CliCommand::Profile { ref path }
        | dyno_cli::CliCommand::Run { ref path } => path.as_ref(),
        dyno_cli::CliCommand::New { .. } => unreachable!(),
        dyno_cli::CliCommand::Help => {
            println!("{}", dyno_cli::usage());
            return;
        }
    };
    let Some(path) = path else {
        run_project_command(command);
        return;
    };
    let mut tune = tune_engine::Tune::new().with_std();

    if matches!(command, dyno_cli::CliCommand::Profile { .. }) {
        match tune.profile_path(path) {
            Ok(report) => {
                print!("{}", dyno_cli::render_profile_report(&report));
                if !report.diagnostics.is_empty() {
                    for diagnostic in &report.diagnostics {
                        eprintln!("{}", tune_diagnostics::render::render_plain(diagnostic));
                    }
                    std::process::exit(1);
                }
                if report.stop_reason.is_some() {
                    std::process::exit(1);
                }
            }
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error(&error) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        return;
    }

    if matches!(command, dyno_cli::CliCommand::Build { .. }) {
        match tune.executable_path(path) {
            Ok(report) => println!("{}", dyno_cli::render_build_report(&report)),
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error(&error) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        return;
    }

    if matches!(command, dyno_cli::CliCommand::Check { .. }) {
        match tune.check_path(path) {
            Ok(report) => {
                for diagnostic in &report.diagnostics {
                    eprintln!("{}", tune_diagnostics::render::render_plain(diagnostic));
                }
                if report.diagnostics.is_empty() {
                    return;
                }
            }
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error(&error) {
                    eprintln!("{diagnostic}");
                }
            }
        }
        std::process::exit(1);
    }

    match tune.run_path(path) {
        Ok(value) => {
            let diagnostics = dyno_cli::render_runtime_boundary_with_sources(&value, tune.db());
            if diagnostics.is_empty() {
                println!("{value:?}");
            } else {
                for diagnostic in diagnostics {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        Err(error) => {
            for diagnostic in dyno_cli::render_engine_error(&error) {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
    }
}

fn run_project_command(command: dyno_cli::CliCommand) {
    let mut tune = tune_engine::Tune::new().with_std();
    let entry = match tune.load_project_manifest("dyno.toml") {
        Ok(entry) => entry,
        Err(error) => {
            for diagnostic in dyno_cli::render_engine_error(&error) {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
    };

    if matches!(command, dyno_cli::CliCommand::Check { .. }) {
        let report = match tune.check_project_entry(entry) {
            Ok(report) => report,
            Err(error) => {
                for diagnostic in dyno_cli::render_engine_error(&error) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        };
        for diagnostic in &report.diagnostics {
            eprintln!("{}", tune_diagnostics::render::render_plain(diagnostic));
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
                for diagnostic in dyno_cli::render_engine_error(&error) {
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
                for diagnostic in dyno_cli::render_engine_error(&error) {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        return;
    }

    match tune.run_project_entry(entry) {
        Ok(value) => {
            let diagnostics = dyno_cli::render_runtime_boundary_with_sources(&value, tune.db());
            if diagnostics.is_empty() {
                println!("{value:?}");
            } else {
                for diagnostic in diagnostics {
                    eprintln!("{diagnostic}");
                }
                std::process::exit(1);
            }
        }
        Err(error) => {
            for diagnostic in dyno_cli::render_engine_error(&error) {
                eprintln!("{diagnostic}");
            }
            std::process::exit(1);
        }
    }
}
