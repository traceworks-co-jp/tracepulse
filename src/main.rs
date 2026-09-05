fn main() {
    if let Err(err) = tracepulse::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
