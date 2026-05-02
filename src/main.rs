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

fn main() -> anyhow::Result<()> {
    let config = Config::parse_cli();

    if config.debug > 0 {
        println!("Debug mode: {}", config.debug);
        println!("Config: {:#?}", config);
    }

    runner::run(config)?;

    Ok(())
}
