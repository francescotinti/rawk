use std::process::Command;

const RAWK: &str = env!("CARGO_BIN_EXE_rawk");

#[test]
fn exit_zero_default() {
    let out = Command::new(RAWK).args(["BEGIN{exit}"]).status().unwrap();
    assert_eq!(out.code(), Some(0), "exit senza arg → 0");
}

#[test]
fn exit_with_value() {
    let out = Command::new(RAWK).args(["BEGIN{exit 7}"]).status().unwrap();
    assert_eq!(out.code(), Some(7), "exit 7 → 7");
}

#[test]
fn exit_from_end_block() {
    let out = Command::new(RAWK)
        .args(["BEGIN{} END{exit 3}"])
        .status()
        .unwrap();
    assert_eq!(out.code(), Some(3), "exit in END → 3");
}

#[test]
fn syntax_error_nonzero() {
    let out = Command::new(RAWK).args(["BEGIN{(("]).output().unwrap();
    assert_ne!(out.status.code(), Some(0), "errore sintassi → non-zero");
}
