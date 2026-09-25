fn main() {
    std::process::exit(dexbot_cli::run(std::env::args().skip(1).collect()));
}
