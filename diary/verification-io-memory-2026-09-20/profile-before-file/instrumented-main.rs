/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

mod io_profile;
mod ast;
mod cli;
mod ere;
mod input;
mod number_format;
mod parser;
mod runner;
mod text;
mod types;
mod unicode_ere;
mod validation;

use cli::Config;

fn main() {
    if std::env::args_os().len() == 1 {
        eprintln!("usage: rawk [-F fs | --csv] [-v var=value] [-f progfile | 'prog'] [file ...]");
        std::process::exit(1);
    }
    let config = Config::parse_cli();

    if config.debug > 0 {
        println!(
            "rawk version {} (debug {})",
            env!("CARGO_PKG_VERSION"),
            config.debug
        );
    }

    let code = match runner::run(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rawk: {e:#}");
            2
        }
    };
    io_profile::report();
    std::process::exit(code);
}
