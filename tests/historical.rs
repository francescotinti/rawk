//! Imported source fixtures are kept in c_awk; no rewriting of their expected output.
use rawk::test_support as harness;
use std::{path::Path, process::Command, time::Duration};

const POSITIVE: &[&str] = &[
    "concat-assign-same",
    "decr-NF",
    "fs-overflow",
    "getline-corruption",
    "getline-numeric",
    "matchop-deref",
    "nf-self-assign",
    "numeric-fs",
    "numeric-output-seps",
    "numeric-rs",
    "numeric-subsep",
    "ofs-rebuild",
    "rs_underflow",
    "rstart-rlength",
    "space",
    "split-fs-from-array",
    "string-conv",
    "system-status",
    "unary-plus",
    "unicode-fs-rs-1",
    "unicode-fs-rs-2",
    "unicode-null-match",
];

fn run_case(name: &str, binary: &Path) -> harness::Outcome {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_awk/bugs-fixed");
    let source = root.join(format!("{name}.awk"));
    let input = root.join(format!("{name}.in"));
    let mut cmd = Command::new(binary);
    cmd.arg("-f").arg(source);
    if input.exists() {
        cmd.arg(input);
    }
    // This fixture constructs a 10,000-byte separator and exercises regex compilation.
    // Debug builds need a larger but still bounded budget than ordinary cases.
    let timeout = if name == "fs-overflow" { 20 } else { 3 };
    harness::run(cmd, b"", Duration::from_secs(timeout)).unwrap()
}

#[test]
fn original_positive_regressions() {
    let reference = harness::reference_binary().unwrap();
    for name in POSITIVE {
        let expected = run_case(name, &reference);
        let actual = run_case(name, Path::new(env!("CARGO_BIN_EXE_rawk")));
        assert!(
            !expected.timed_out && expected.code == Some(0),
            "{name}: {expected:?}"
        );
        assert_eq!(actual, expected, "{name}");
    }
}

#[test]
fn original_invalid_programs_are_rejected_without_panicking() {
    let reference = harness::reference_binary().unwrap();
    // Diagnostic wording is implementation-specific; both must fail with a diagnostic.
    for name in [
        "pfile-overflow",
        "repetition-no-atom",
        "missing-precision",
        "negative-nf",
    ] {
        for binary in [&reference, Path::new(env!("CARGO_BIN_EXE_rawk"))] {
            let outcome = run_case(name, binary);
            assert_eq!(outcome.code, Some(2), "{name}: {outcome:?}");
            assert!(!outcome.timed_out && outcome.stdout.is_empty() && !outcome.stderr.is_empty());
            assert!(!String::from_utf8_lossy(&outcome.stderr).contains("panicked"));
        }
    }
}
