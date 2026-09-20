use rawk::test_support as harness;
use std::{path::Path, process::Command, time::Duration};

#[test]
fn original_cli_and_array_shell_drivers() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/driver_audit.py");
    let mut command = Command::new("python3");
    command
        .arg(script)
        .args(["--rawk", env!("CARGO_BIN_EXE_rawk"), "--output"])
        .arg(output.path());
    let result = harness::run(command, b"", Duration::from_secs(20)).unwrap();
    assert_eq!(result.code, Some(0), "{result:?}");
    assert!(!result.timed_out && result.stderr.is_empty(), "{result:?}");
}

#[test]
fn admitted_original_shell_drivers() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/full_driver_audit.py");
    let mut command = Command::new("python3");
    command
        .arg(script)
        .args([
            "--rawk",
            env!("CARGO_BIN_EXE_rawk"),
            "--verified",
            "--check",
            "--output",
        ])
        .arg(output.path());
    let result = harness::run(command, b"", Duration::from_secs(120)).unwrap();
    assert_eq!(result.code, Some(0), "{result:?}");
    assert!(!result.timed_out && result.stderr.is_empty(), "{result:?}");
}

#[test]
fn shell_driver_gate_rejects_missing_interpreter_output() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/full_driver_audit.py");
    let mut command = Command::new("python3");
    command
        .arg(script)
        .args([
            "--rawk",
            "/usr/bin/true",
            "--drivers",
            "T.delete",
            "--check",
            "--output",
        ])
        .arg(output.path());
    let result = harness::run(command, b"", Duration::from_secs(10)).unwrap();
    assert_eq!(result.code, Some(1), "{result:?}");
    assert!(!result.timed_out && result.stderr.is_empty(), "{result:?}");
}
