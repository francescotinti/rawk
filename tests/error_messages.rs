use std::process::Command;

const RAWK: &str = env!("CARGO_BIN_EXE_rawk");

#[test]
fn missing_program_file_mentions_filename() {
    let out = Command::new(RAWK)
        .args(["-f", "/nonexistent/path/foo.awk"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("/nonexistent/path/foo.awk"),
        "stderr deve nominare il file mancante; got: {stderr}"
    );
    assert!(
        stderr.contains("programfile") || stderr.contains("lettura"),
        "stderr deve descrivere l'operazione; got: {stderr}"
    );
}

#[test]
fn output_to_unwritable_path_mentions_filename() {
    let out = Command::new(RAWK)
        .args(["BEGIN { print \"x\" >> \"/nonexistent/dir/out\" }"])
        .output()
        .unwrap();
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("/nonexistent/dir/out"),
        "stderr deve nominare il path output; got: {stderr}"
    );
}
