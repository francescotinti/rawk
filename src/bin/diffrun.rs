use anyhow::Result;
use clap::Parser;
use rawk::test_support as harness;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
struct Config {
    #[arg(default_value = "tests/testsuite.xml")]
    manifest: PathBuf,
    #[arg(long)]
    awk: Option<PathBuf>,
    #[arg(long)]
    rawk: Option<PathBuf>,
    #[arg(long, default_value_t = 3000)]
    timeout_ms: u64,
}

fn run() -> Result<bool> {
    let config = Config::parse();
    let reference = match config.awk {
        Some(path) => path.canonicalize()?,
        None => harness::reference_binary()?,
    };
    let rawk = config
        .rawk
        .unwrap_or(std::env::current_exe()?.with_file_name("rawk"))
        .canonicalize()?;
    let cases = harness::load(&config.manifest)?;
    println!("Reference awk: {}", reference.display());
    println!("Total testcase: {}", cases.len());
    let mut matched = 0;
    let mut expected = 0;
    let mut unexpected = 0;
    for case in cases {
        let timeout = Duration::from_millis(config.timeout_ms);
        let actual = harness::execute(&rawk, &case, timeout)?;
        let oracle = harness::execute(&reference, &case, timeout)?;
        if !harness::validate(&case, &actual) {
            unexpected += 1;
            println!(
                "FAIL {}: rawk violates its expected result: {:?}",
                case.name, actual
            );
        } else if harness::equivalent(&case, &actual, &oracle) {
            matched += 1;
        } else if harness::expected_divergence(&case, &oracle, &reference) {
            expected += 1;
        } else {
            unexpected += 1;
            println!(
                "FAIL {}: reference {:?}; rawk {:?}",
                case.name, oracle, actual
            );
        }
    }
    println!(
        "  MATCH: {matched}\n  EXPECTED-DIVERGE: {expected}\n  UNEXPECTED-DIVERGE: {unexpected}\n  SKIPPED: 0"
    );
    Ok(unexpected == 0)
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::FAILURE,
        Err(error) => {
            eprintln!("diffrun: {error:#}");
            std::process::ExitCode::from(2)
        }
    }
}
