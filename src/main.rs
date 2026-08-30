fn main() {
    if let Err(err) = trace_pulse::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
