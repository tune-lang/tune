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
    let path = match command {
        dyno_cli::CliCommand::Check { ref path } | dyno_cli::CliCommand::Run { ref path } => path,
        dyno_cli::CliCommand::Help => {
            println!("{}", dyno_cli::usage());
            return;
        }
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            std::process::exit(1);
        }
    };

    let mut tune = tune_engine::Tune::new();
    let file = match tune.add_file(path.clone(), source) {
        Some(file) => file,
        None => {
            eprintln!("failed to allocate source file");
            std::process::exit(1);
        }
    };

    if matches!(command, dyno_cli::CliCommand::Check { .. }) {
        let Some(report) = tune.check_file(file) else {
            eprintln!("failed to check {path}");
            std::process::exit(1);
        };
        for diagnostic in &report.diagnostics {
            eprintln!("{}", tune_diagnostics::render::render_plain(diagnostic));
        }
        if report.diagnostics.is_empty() {
            return;
        }
        std::process::exit(1);
    }

    match tune.run_file(file) {
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
