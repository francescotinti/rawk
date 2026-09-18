use rawk::test_support::{self as harness, Outcome};
use std::process::Command;
use std::time::Duration;

#[test]
fn preserves_bytes_whitespace_and_status() {
    let mut cmd = Command::new("/bin/sh");
    cmd.args([
        "-c",
        "printf ' \\377\\000\\n'; printf 'error\\n' >&2; exit 7",
    ]);
    let out = harness::run(cmd, b"", Duration::from_secs(2)).unwrap();
    assert_eq!(out.stdout, b" \xff\0\n");
    assert_eq!(out.stderr, b"error\n");
    assert_eq!(out.code, Some(7));
    assert!(!harness::output_matches(b"x\n", b"x", "exact"));
    assert!(!harness::output_matches(
        b"x\nx\n",
        b"x\n",
        "unordered-lines"
    ));
}

#[test]
fn timeout_kills_descendants() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("escaped");
    let mut cmd = Command::new("/bin/sh");
    cmd.args(["-c", "(sleep 0.3; touch \"$1\") & wait", "sh"])
        .arg(&marker);
    let out = harness::run(cmd, b"", Duration::from_millis(30)).unwrap();
    assert!(out.timed_out);
    std::thread::sleep(Duration::from_millis(400));
    assert!(!marker.exists());
}

#[test]
fn each_run_has_a_fresh_directory() {
    for _ in 0..2 {
        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "test ! -e shared && touch shared"]);
        assert_eq!(
            harness::run(cmd, b"", Duration::from_secs(2)).unwrap().code,
            Some(0)
        );
    }
}

#[test]
fn annotation_does_not_accept_new_failures() {
    let case: harness::Case = quick_xml::de::from_str(
        r#"<testcase name="example">
      <awk>BEGIN{print 1}</awk><expected_stdout>1
</expected_stdout><expected_divergence reason="example"><stdout>2
</stdout><stderr></stderr><status>0</status></expected_divergence></testcase>"#,
    )
    .unwrap();
    let mut out = Outcome {
        stdout: b"2\n".to_vec(),
        stderr: vec![],
        code: Some(0),
        timed_out: false,
    };
    assert!(harness::expected_divergence(
        &case,
        &out,
        std::path::Path::new("awk")
    ));
    out.code = Some(101);
    assert!(!harness::expected_divergence(
        &case,
        &out,
        std::path::Path::new("awk")
    ));
    assert!(!harness::validate(&case, &out));
}

#[test]
fn aggregate_gate_reports_an_intermediate_failure() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/checks.sh");
    let out = Command::new("bash")
        .args([
            "-c",
            r#"
source "$1"
for fn in $(declare -F | awk '$3 ~ /^check_/ {print $3}'); do eval "$fn() { return 0; }"; done
check_fmt() { return 1; }
run_all
"#,
            "bash",
        ])
        .arg(script)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}
