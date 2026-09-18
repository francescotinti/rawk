mod common;

#[test]
fn regex_uses_leftmost_longest() {
    assert_eq!(
        common::run_with_stdin(
            "BEGIN{match(\"ab\",/a|ab/); print RSTART,RLENGTH; match(\"xab\",/a|ab/); print RSTART,RLENGTH}",
            b""
        ),
        b"1 2\n2 2\n"
    );
}
#[test]
fn split_preserves_empty_fields_and_clears_array() {
    assert_eq!(
        common::run_with_stdin(
            "BEGIN{a[9]=\"old\"; print split(\"a,,b\",a,\",\"); print a[2],(9 in a)}",
            b""
        ),
        b"3\n 0\n"
    );
}
#[test]
fn substitutions_report_count_not_changed_text() {
    assert_eq!(
        common::run_with_stdin(
            "BEGIN{s=\"aaa\"; print gsub(/a/,\"b\",s),s; print sub(/b/,\"b\",s)}",
            b""
        ),
        b"3 bbb\n1\n"
    );
}
#[test]
fn fields_accept_regex_separator() {
    assert_eq!(
        common::run_with_stdin("BEGIN{FS=\"[,:]+\"} {print NF,$1,$2,$3}", b"a,b:c\n"),
        b"3 a b c\n"
    );
}

#[test]
fn invalid_regex_is_reported() {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rawk"));
    cmd.arg("BEGIN{r=\"[\"; print (\"a\" ~ r)}");
    let out = rawk::test_support::run(cmd, b"", std::time::Duration::from_secs(2)).unwrap();
    assert_eq!(out.code, Some(2));
    assert!(out.stdout.is_empty());
}

#[test]
fn csv_handles_quoted_delimiters_newlines_and_escaped_quotes() {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_rawk"));
    cmd.args(["--csv", "{print NF; print $2}"]);
    let out = rawk::test_support::run(
        cmd,
        b"a,\"b,c\",d\r\na,\"b\r\nc\",d\r\na,\"b\"\"c\",d\n",
        std::time::Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(out.stdout, b"3\nb,c\n3\nb\nc\n3\nb\"c\n");
    assert_eq!(out.code, Some(0));
}
