use rawk::test_support::{Outcome, run};
use std::process::Command;
use std::time::Duration;

fn execute(args: &[&str], input: &[u8]) -> Outcome {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rawk"));
    cmd.args(args);
    run(cmd, input, Duration::from_secs(2)).unwrap()
}

#[test]
fn file_program_keeps_all_input_files() {
    let temp = tempfile::tempdir().unwrap();
    let program = temp.path().join("program.awk");
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    std::fs::write(&program, "{print $0}").unwrap();
    std::fs::write(&first, "first\n").unwrap();
    std::fs::write(&second, "second\n").unwrap();
    let out = execute(
        &[
            "-f",
            program.to_str().unwrap(),
            first.to_str().unwrap(),
            second.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(out.stdout, b"first\nsecond\n");
    assert_eq!(out.code, Some(0));
}

#[test]
fn safe_mode_rejects_even_unexecuted_forbidden_operations() {
    for program in [
        "BEGIN {system(\"printf BAD\")}",
        "BEGIN {if(0) print 1 > \"out\"}",
        "function f(){print 1 | \"cat\"} BEGIN {print 1}",
        "BEGIN {\"printf BAD\" | getline x}",
    ] {
        let out = execute(&["--safe", program], b"");
        assert_eq!(out.code, Some(2), "{out:?}");
        assert!(out.stdout.is_empty());
    }
    let out = execute(&["-safe", "BEGIN {print (\"PATH\" in ENVIRON)}"], b"");
    assert_eq!(out.stdout, b"0\n");
}

#[test]
fn invalid_builtin_arity_is_a_language_error() {
    for program in [
        "BEGIN {print substr(\"a\")}",
        "BEGIN {print sin()}",
        "BEGIN {print split(\"a\")}",
    ] {
        let out = execute(&[program], b"");
        assert_eq!(out.code, Some(2), "{out:?}");
        assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    }
}

#[test]
fn malformed_formats_and_invalid_runtime_arguments_do_not_panic() {
    for program in [
        "BEGIN{printf \"%*s\"}",
        "BEGIN{printf \"%\\xffs\",1}",
        "BEGIN{print strftime(\"%Q\")}",
        "BEGIN{print lshift(1,64)}",
        "BEGIN{NF=-5}",
    ] {
        let out = execute(&[program], b"");
        assert_eq!(out.code, Some(2), "{program}: {out:?}");
        assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    }
}

#[test]
fn arrays_and_scalars_cannot_be_interchanged() {
    for program in [
        "BEGIN{a=3; a[1]=2}",
        "BEGIN{a[1]=2; print a}",
        "function f(a){a[1]=2} BEGIN{x=3;f(x)}",
    ] {
        let out = execute(&[program], b"");
        assert_eq!(out.code, Some(2), "{out:?}");
    }
}
