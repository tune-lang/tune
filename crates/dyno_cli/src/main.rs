fn main() {
    let mut tune = tune_engine::Tune::new();
    let status = if tune.check_source("<dyno-cli>", "").is_some() {
        "ready"
    } else {
        "unavailable"
    };

    println!("dyno cli skeleton: engine {status}");
}
