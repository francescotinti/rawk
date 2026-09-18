mod common;

#[test]
fn getline_uses_main_stream_and_counts_every_read() {
    assert_eq!(
        common::run_with_stdin("{getline x; print $0,x,NR,FNR}", b"a\nb\nc\nd\n"),
        b"a b 2 2\nc d 4 4\n"
    );
}

#[test]
fn rewriting_record_does_not_count_a_read() {
    assert_eq!(
        common::run_with_stdin(
            "{$0=$0; gsub(/z/,\"x\"); print NR,FNR} END{print NR}",
            b"a\nb\n"
        ),
        b"1 1\n2 2\n2\n"
    );
}

#[test]
fn rs_changes_apply_to_unconsumed_input() {
    assert_eq!(
        common::run_with_stdin("{print $0; RS=\":\"}", b"a\nb:c:"),
        b"a\nb\nc\n"
    );
}

#[test]
fn missing_getline_file_is_an_error_not_eof() {
    assert_eq!(
        common::run_with_stdin(
            "BEGIN{print (getline x < \"/nonexistent/rawk-absent\")}",
            b""
        ),
        b"-1\n"
    );
}

#[test]
fn file_getline_and_interleaved_assignments() {
    let dir = tempfile::tempdir().unwrap();
    let a = dir.path().join("a");
    let b = dir.path().join("b");
    std::fs::write(&a, b"a\nb\n").unwrap();
    std::fs::write(&b, b"c\nd\n").unwrap();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rawk"));
    cmd.args([
        "-v",
        "x=0",
        "BEGIN{print x} {getline v;print x,$0,v,NR,FNR}",
        "x=1",
    ])
    .arg(a)
    .arg("x=2")
    .arg(b);
    let out = rawk::test_support::run(cmd, b"", std::time::Duration::from_secs(2)).unwrap();
    assert_eq!(out.stdout, b"0\n1 a b 2 2\n2 c d 4 2\n");
    assert_eq!(out.code, Some(0));
}
