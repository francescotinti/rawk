use std::process::Command;
use std::time::Duration;

pub fn run_with_stdin(prog: &str, stdin: &[u8]) -> Vec<u8> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rawk"));
    command.arg(prog);
    let out = rawk::test_support::run(command, stdin, Duration::from_secs(3)).unwrap();
    assert!(
        !out.timed_out && out.code == Some(0) && out.stderr.is_empty(),
        "{out:?}"
    );
    out.stdout
}
