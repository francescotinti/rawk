/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

mod ast;
mod cli;
mod parser;
mod runner;
mod types;

use cli::Config;

fn main() {
    let config = Config::parse_cli();

    if config.debug > 0 {
        eprintln!("Debug mode: {}", config.debug);
        eprintln!("Config: {:#?}", config);
    }

    let code = match runner::run(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rawk: {e:#}");
            2
        }
    };
    std::process::exit(code);
}
