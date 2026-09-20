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

fn compare_fields(program: &str, input: &[u8]) {
    let run = |binary: &std::path::Path| {
        let mut cmd = std::process::Command::new(binary);
        cmd.arg(program);
        rawk::test_support::run(cmd, input, std::time::Duration::from_secs(3)).unwrap()
    };
    let expected = run(&rawk::test_support::reference_binary().unwrap());
    assert_eq!(expected.code, Some(0));
    assert_eq!(
        run(std::path::Path::new(env!("CARGO_BIN_EXE_rawk"))),
        expected,
        "{program}"
    );
}

#[test]
fn successive_whitespace_records_preserve_values_and_numeric_types() {
    // Saved scalars/array keys must not alias reused storage; numeric field
    // assignments, shorter records and empty records must not leave old data.
    compare_fields(
        r#"{print NF,"["$1"]",($1==2),($1=="02"),$1+0,previous;
            a[$1]++; previous=$1; $1=42; $4="tail"; NF=2}
           END{print a["02"],a["word"],a[""],previous}"#,
        b"02 3 4 5\nword\n \t \n-0 +2 .5 2e3\n1e-999 1e999\n\xff 2\n",
    );
}

#[test]
fn whitespace_reuse_handles_resplit_getline_and_separator_changes() {
    compare_fields(
        r#"{print NR,NF,$1,$2; FS=","; $0="a,,b"; print NF,"["$2"]";
            FS=" "; $0="  02\tword\nlast  "; print NF,$1,$2,$3;
            getline; print NR,NF,$1,$2; saved=$2; $2=7; print saved,$2}"#,
        b"one two\nthree four five\nsix\nseven eight\n",
    );
    compare_fields(
        r#"BEGIN{RS=""} {print NF,$1,$2,$3,$4}"#,
        b"  one\ttwo\nthree\n\n \t\n\nfour\nfive\n",
    );
}

#[test]
fn reused_whitespace_fields_preserve_nul_bytes() {
    // NUL retention is deliberate and cannot use the C oracle's truncation.
    assert_eq!(
        common::run_with_stdin(
            r#"{print NF,"["$1"]","["$2"]",saved; saved=$1; $1=99}"#,
            b"a\0b x\nz y\n\0 q\n",
        ),
        b"2 [a\0b] [x] \n2 [z] [y] a\0b\n2 [\0] [q] z\n"
    );
}
