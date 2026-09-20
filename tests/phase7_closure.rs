use rawk::test_support as h;
use std::{os::unix::ffi::OsStringExt, path::Path, process::Command, time::Duration};

fn compare(program: &[u8], input: &[u8]) {
    let run = |binary| {
        let mut command = Command::new(binary);
        command.arg(std::ffi::OsString::from_vec(program.to_vec()));
        h::run(command, input, Duration::from_secs(3)).unwrap()
    };
    let c = run(h::reference_binary().unwrap());
    let rust = run(std::path::PathBuf::from(env!("CARGO_BIN_EXE_rawk")));
    assert!(!c.timed_out && !rust.timed_out);
    assert_eq!(rust, c, "program: {}", String::from_utf8_lossy(program));
}

#[test]
fn individual_driver_contracts() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let mut command = Command::new("python3");
    command
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/closure_cases.py"))
        .args(["--rawk", env!("CARGO_BIN_EXE_rawk"), "--check", "--output"])
        .arg(output.path());
    let result = h::run(command, b"", Duration::from_secs(60)).unwrap();
    assert_eq!(result.code, Some(0), "{result:?}");
    assert!(!result.timed_out && result.stderr.is_empty());
}

#[test]
fn contracts_reject_mutations() {
    let mut command = Command::new("python3");
    command.arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/test_closure_contracts.py"));
    let result = h::run(command, b"", Duration::from_secs(10)).unwrap();
    assert_eq!(result.code, Some(0), "{result:?}");
    assert!(!result.timed_out);
}

#[test]
fn raw_source_bytes_survive_inline_file_and_stdin() {
    let mut program = b"BEGIN {s=\"".to_vec();
    program.extend(128..=255);
    program.extend_from_slice(b"\"; print length(s),s; print s ~ /\xff/}");
    compare(&program, b"");
    for binary in [
        h::reference_binary().unwrap(),
        env!("CARGO_BIN_EXE_rawk").into(),
    ] {
        let mut command = Command::new(&binary);
        command.args(["-f", "bytes.awk"]);
        let file = h::run_with_fixtures(
            command,
            b"",
            Duration::from_secs(3),
            &[("bytes.awk", &program)],
        )
        .unwrap()
        .0;
        let mut command = Command::new(binary);
        command.args(["-f", "-"]);
        let stdin = h::run(command, &program, Duration::from_secs(3)).unwrap();
        assert_eq!(file, stdin);
        assert_eq!(file.code, Some(0));
    }
    // The retained unknown-escape extension must preserve the original high
    // byte, not turn a synthetic source escape into literal ASCII "xFF".
    let mut command = Command::new(env!("CARGO_BIN_EXE_rawk"));
    command.arg(std::ffi::OsString::from_vec(
        b"BEGIN { print \"\\\xff\" }".to_vec(),
    ));
    let out = h::run(command, b"", Duration::from_secs(3)).unwrap();
    assert_eq!(out.stdout, b"\\\xff\n");
    assert_eq!(out.code, Some(0));
}

#[test]
fn generated_octal_regex_atoms_and_literal_operators() {
    let mut program = String::from("BEGIN {\n");
    for byte in 1..=255 {
        program.push_str(&format!(
            "s=sprintf(\"%c\",{byte}); print {byte},s ~ /\\{byte:03o}/\n"
        ));
    }
    program.push('}');
    compare(program.as_bytes(), b"");
    for pattern in [
        "{", "a{b", ")", r"\b", r"\8", r"\xQ", r"\uQ", r"\052", "[]a-]",
    ] {
        let program = format!("{{print $0 ~ /{pattern}/}}");
        compare(program.as_bytes(), b"a{b\n)\n*\n8\n\x08\nxQ\nuQ\n]\n-\n");
    }
}

#[test]
fn keyword_prefixes_and_integral_precision() {
    compare(b"BEGIN {break_after=2;continue_count=3;nextfilex=4;returning=5;exitcode=6;printfx=7;print break_after,continue_count,nextfilex,returning,exitcode,printfx}", b"");
    for power in [16, 20, 21, 25, 29, 30, 40, 100, 300] {
        let program =
            format!("BEGIN {{CONVFMT=OFMT=\"%.2f\";x=1e{power};print x,x+0;print \"<\" x \">\"}}");
        compare(program.as_bytes(), b"");
    }
}

#[test]
fn separators_keep_string_and_regex_semantics() {
    for sep in [r#"" ""#, "/ /", "//", r#""""#, "/./", r#"".""#] {
        let program =
            format!("{{n=split($0,a,{sep});print n;for(i=1;i<=n;i++)print \"<\" a[i] \">\"}}");
        compare(program.as_bytes(), b" a  b\tc \nabc\n\n");
    }
    compare(b"BEGIN {RS=\"^a\"} {print NR,length($0),$0}", b"aaa1a2a\n");
    compare(b"{print;RS=\"^a\"}", b"first\naaa\naaa\n");
}

#[test]
fn closed_output_pipe_reports_language_error() {
    // Close the read end before exec to make EPIPE deterministic. Python keeps
    // SIGPIPE ignored with restore_signals=False, as the historical shell test.
    let script = r#"
import os, subprocess, sys
for binary in sys.argv[1:]:
    reader, writer = os.pipe()
    os.close(reader)
    try:
        result = subprocess.run([binary, 'BEGIN { print "hi" }'], stdout=writer,
                                stderr=subprocess.PIPE, restore_signals=False, timeout=3)
        assert result.returncode == 2, (binary, result.returncode, result.stderr)
        assert result.stderr, binary
    finally:
        os.close(writer)
"#;
    let mut command = Command::new("python3");
    command
        .args(["-c", script, env!("CARGO_BIN_EXE_rawk")])
        .arg(h::reference_binary().unwrap());
    let result = h::run(command, b"", Duration::from_secs(10)).unwrap();
    assert_eq!(result.code, Some(0), "{result:?}");
    assert!(!result.timed_out);
}
