/*
 * Project: rawk (Rust AWK)
 * Authors: Francesco Tinti & Antigravity (Google Deepmind)
 * Description: A high-fidelity port of the historic AWK language from C to Rust.
 */

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Rust port of One True Awk", long_about = None)]
pub struct Config {
    /// Set field separator
    #[arg(short = 'F')]
    pub field_separator: Option<String>,

    /// Turn on CSV input processing
    #[arg(long = "csv")]
    pub csv: bool,

    /// Assign values to variables (e.g., var=value)
    #[arg(short = 'v')]
    pub variables: Vec<String>,

    /// Read program from a file
    #[arg(short = 'f')]
    pub program_files: Vec<String>,

    /// Enable "safe" mode
    #[arg(short = 's', long = "safe")]
    pub safe: bool,

    /// Debug level
    #[arg(short = 'd', default_value = "0")]
    pub debug: u8,

    /// The inline awk program (if no -f is provided)
    #[arg(required_unless_present = "program_files")]
    pub program: Option<String>,

    /// Input files to process
    pub input_files: Vec<String>,
}

impl Config {
    pub fn parse_cli() -> Self {
        Config::parse()
    }
}
