use rawk::test_support as harness;
use std::{process::Command, time::Duration};

#[test]
fn stdin_program_and_empty_statements() {
    for (args, input) in [
        (vec!["-f", "-"], b"BEGIN { print 42 }".as_slice()),
        (
            vec!["BEGIN {;;; f();exit}; END {print 7};function f(){;}"],
            b"".as_slice(),
        ),
        (vec!["-F", "t", "{ print NF }"], b"a\tb\tc\n".as_slice()),
    ] {
        let run = |binary| {
            let mut cmd = Command::new(binary);
            cmd.args(&args);
            harness::run(cmd, input, Duration::from_secs(3)).unwrap()
        };
        assert_eq!(
            run(std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk"))),
            run(harness::reference_binary().unwrap())
        );
    }
}

#[test]
fn multifile_syntax_error_names_offending_source() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rawk"));
    command.args(["-f", "first.awk", "-f", "second.awk", "-f", "third.awk"]);
    let (result, _) = harness::run_with_fixtures(
        command,
        b"",
        Duration::from_secs(3),
        &[
            ("first.awk", b"BEGIN {"),
            ("second.awk", b"print 1\n]"),
            ("third.awk", b"}"),
        ],
    )
    .unwrap();
    assert_eq!(result.code, Some(2));
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("syntax error in file second.awk"),
        "{result:?}"
    );
}

#[test]
fn redirected_stdin_shares_buffer_with_main_input() {
    for program in [
        r#"BEGIN { while (getline < "-") print; print NR }"#,
        r#"BEGIN { getline; print; getline x < "-"; print x; getline; print NR, $0 }"#,
        r#"BEGIN { getline x < "-"; print x } { print NR, $0 }"#,
    ] {
        let run = |binary| {
            let mut cmd = Command::new(binary);
            cmd.arg(program);
            harness::run(cmd, b"one\ntwo\nthree\n", Duration::from_secs(3)).unwrap()
        };
        let c = run(harness::reference_binary().unwrap());
        let rust = run(std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk")));
        assert!(!c.timed_out && !rust.timed_out);
        assert_eq!(rust, c, "{program}");
    }
}

#[test]
fn invalid_function_parameters_are_errors_before_execution() {
    for program in [
        "function f(f) { f() } BEGIN { f() }",
        "function f(x,x) {} BEGIN {print 42}",
    ] {
        for binary in [
            harness::reference_binary().unwrap(),
            std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk")),
        ] {
            let mut cmd = Command::new(binary);
            cmd.arg(program);
            let result = harness::run(cmd, b"", Duration::from_secs(3)).unwrap();
            assert_eq!(result.code, Some(2), "{result:?}");
            assert!(!result.timed_out && result.stdout.is_empty());
        }
    }
}

#[test]
fn getline_comparison_is_not_part_of_filename() {
    let run = |binary| {
        let mut cmd = Command::new(binary);
        cmd.arg(r#"BEGIN { while (getline < "records" > 0) print; print NR }"#);
        harness::run_with_fixtures(cmd, b"", Duration::from_secs(3), &[("records", b"a\nb\n")])
            .unwrap()
            .0
    };
    let c = run(harness::reference_binary().unwrap());
    let rust = run(std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk")));
    assert!(!c.timed_out && !rust.timed_out);
    assert_eq!(rust, c);
    assert_eq!(rust.stdout, b"a\nb\n0\n");
}

#[test]
fn field_index_beyond_reference_integer_range_is_an_error() {
    for binary in [
        harness::reference_binary().unwrap(),
        std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk")),
    ] {
        let mut cmd = Command::new(binary);
        cmd.arg("{print $40000000000000}");
        let result = harness::run(cmd, b"x\n", Duration::from_secs(3)).unwrap();
        assert_eq!(result.code, Some(2), "{result:?}");
        assert!(result.stdout.is_empty() && !result.timed_out);
        assert!(String::from_utf8_lossy(&result.stderr).contains("out of range field"));
    }
}
