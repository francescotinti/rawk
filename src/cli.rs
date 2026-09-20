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
    pub program: Option<std::ffi::OsString>,

    /// Input files to process
    pub input_files: Vec<String>,
}

impl Config {
    pub fn parse_cli() -> Self {
        let mut args = Vec::new();
        let mut options = true;
        let mut value_pending = false;
        for (i, arg) in std::env::args_os().enumerate() {
            if i == 0 || value_pending {
                value_pending = false;
                args.push(arg);
                continue;
            }
            let text = arg.to_string_lossy();
            if options && text == "-safe" {
                args.push("--safe".into());
            } else if options && text == "-d" {
                args.push("-d1".into());
            } else if options && text.starts_with('-') && text != "-" {
                if text == "--" {
                    options = false;
                } else if ["-f", "-F", "-v"].contains(&text.as_ref()) {
                    value_pending = true;
                } else if !["--csv", "--safe", "--help", "--version", "-h", "-V", "-s"]
                    .contains(&text.as_ref())
                    && !["-f", "-F", "-v", "-d"].iter().any(|p| text.starts_with(p))
                {
                    eprintln!("rawk: unknown option {text} ignored");
                    continue;
                }
                args.push(arg);
            } else {
                options = false;
                args.push(arg);
            }
        }
        let mut config = Config::parse_from(args);
        if !config.program_files.is_empty()
            && let Some(first_input) = config.program.take()
        {
            config
                .input_files
                .insert(0, first_input.to_string_lossy().into_owned());
        }
        config
    }
}
