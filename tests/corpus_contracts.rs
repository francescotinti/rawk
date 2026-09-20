use rawk::test_support as h;
use std::{path::Path, process::Command, time::Duration};

#[test]
fn entire_small_corpus_obeys_explicit_contracts() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let mut cmd = Command::new("python3");
    cmd.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/corpus_audit.py"))
        .args(["--rawk", env!("CARGO_BIN_EXE_rawk"), "--check", "--output"])
        .arg(output.path());
    let out = h::run(cmd, b"", Duration::from_secs(30)).unwrap();
    assert_eq!(out.code, Some(0), "{out:?}");
    assert!(out.stderr.is_empty() && !out.timed_out);
}

#[test]
fn rng_is_bounded_repeatable_and_srand_returns_previous_seed() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rawk"));
    cmd.arg("BEGIN {srand(17);for(i=0;i<1000;i++){a[i]=rand();if(a[i]<0||a[i]>=1)exit 3} print srand(17);for(i=0;i<1000;i++)if(a[i]!=rand())exit 4;print srand(23),srand(31)}");
    let out = h::run(cmd, b"", Duration::from_secs(3)).unwrap();
    assert_eq!(out.code, Some(0));
    assert_eq!(out.stdout, b"17\n17 23\n");
    assert!(out.stderr.is_empty());
}

#[test]
fn contracts_reject_new_differences() {
    let mut cmd = Command::new("python3");
    cmd.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/test_corpus_contracts.py"))
        .env("RAWK_BINARY", env!("CARGO_BIN_EXE_rawk"));
    let out = h::run(cmd, b"", Duration::from_secs(10)).unwrap();
    assert_eq!(out.code, Some(0), "{out:?}");
    assert!(!out.timed_out);
}
