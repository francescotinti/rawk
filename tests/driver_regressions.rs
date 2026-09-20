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

#[test]
fn standard_output_aliases_share_streams_and_lifecycle() {
    for program in [
        r#"BEGIN { print "normal"; print "redirect" > "/dev/stdout"; print "after" }"#,
        r#"BEGIN { printf "normal"; printf "redirect" > "/dev/stdout"; print "after" }"#,
        r#"BEGIN { print "normal"; print "append" >> "/dev/stdout"; print "after" }"#,
        r#"BEGIN { print "first" > "/dev/stderr"; print "second" >> "/dev/stderr"; print "last" > "/dev/stderr" }"#,
        r#"BEGIN { print "before"; s=close("/dev/stdout"); print s > "status"; print "hidden"; print "reopened" > "/dev/stdout" }"#,
        r#"BEGIN { s=close("/dev/stderr"); print s; print "hidden" > "/dev/stderr" }"#,
        r#"BEGIN { printf "before"; print fflush("/dev/stdout"); print "after" > "/dev/stdout" }"#,
        r#"BEGIN { print fflush("/dev/stderr"); print "after" > "/dev/stderr" }"#,
        r#"BEGIN { close("/dev/stdout"); print system("echo hidden") > "status" }"#,
        r#"BEGIN { close("/dev/stderr"); print system("echo hidden >&2") }"#,
        r#"BEGIN { print close("/dev/stderr"), close("/dev/stderr"), fflush("/dev/stderr"); print "hidden" > "/dev/stderr"; print fflush("/dev/stderr"), close("/dev/stderr") }"#,
        r#"BEGIN { printf "%c%c",0,255; printf "%c%c",255,0 > "/dev/stdout"; print "end" }"#,
    ] {
        let run = |binary| {
            let mut command = Command::new(binary);
            command.arg(program);
            harness::run_with_fixtures(command, b"", Duration::from_secs(3), &[]).unwrap()
        };
        let c = run(harness::reference_binary().unwrap());
        let rust = run(std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk")));
        assert!(!c.0.timed_out && !rust.0.timed_out);
        assert_eq!(rust, c, "{program}");
    }
}

#[test]
fn checked_math_matches_native_oracle() {
    for (name, values) in [
        ("log", vec!["-1", "0", "-0", "1", "2", "1e-300", "1e300"]),
        (
            "exp",
            vec![
                "-1000", "-745", "-710", "-1", "0", "1", "709", "710", "1000",
            ],
        ),
        ("sqrt", vec!["-1", "-0", "0", "1", "2", "1e-300", "1e300"]),
    ] {
        for value in values {
            let program = format!("BEGIN {{ print {name}({value}); print {name}(1) }}");
            let run = |binary| {
                let mut command = Command::new(binary);
                command.arg(&program);
                harness::run(command, b"", Duration::from_secs(3)).unwrap()
            };
            let c = run(harness::reference_binary().unwrap());
            let rust = run(std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk")));
            assert!(!c.timed_out && !rust.timed_out);
            assert_eq!((rust.code, &rust.stdout), (c.code, &c.stdout), "{program}");
            // Error wording/source context differs deliberately; the warning
            // class and number of occurrences must still match the oracle.
            for message in ["argument out of domain", "result out of range"] {
                let warning = format!("{name} {message}");
                assert_eq!(
                    String::from_utf8_lossy(&rust.stderr)
                        .matches(&warning)
                        .count(),
                    String::from_utf8_lossy(&c.stderr).matches(&warning).count(),
                    "{program}: C={c:?}, Rust={rust:?}"
                );
            }
            assert_eq!(rust.stderr.is_empty(), c.stderr.is_empty(), "{program}");
        }
    }
}
