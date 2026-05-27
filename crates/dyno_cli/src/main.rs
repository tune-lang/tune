fn main() {
    let Some(path) = std::env::args().nth(1) else {
        println!("dyno cli skeleton: pass a Tune source file to run it");
        return;
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            std::process::exit(1);
        }
    };

    let mut tune = tune_engine::Tune::new();
    let file = match tune.add_file(path, source) {
        Some(file) => file,
        None => {
            eprintln!("failed to allocate source file");
            std::process::exit(1);
        }
    };

    match tune.run_file(file) {
        Ok(value) => println!("{value:?}"),
        Err(error) => {
            eprintln!("execution failed: {error:?}");
            std::process::exit(1);
        }
    }
}
