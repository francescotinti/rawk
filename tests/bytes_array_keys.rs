//! Phase 7.3 acceptance — chiavi array byte-aware.
//!
//! RED prima della migrazione 7.3 (le chiavi `0x80..0xFF` collidono dopo
//! `String::from_utf8_lossy` perché tutti i byte invalidi diventano `\u{FFFD}`).
//! GREEN dopo 7.3.2 (storage `HashMap<Vec<u8>, AwkValue>` byte-pulito).

use std::io::Write;
use std::process::{Command, Stdio};

const RAWK: &str = env!("CARGO_BIN_EXE_rawk");

fn run_with_stdin(prog: &str, stdin: &[u8]) -> Vec<u8> {
    let mut child = Command::new(RAWK)
        .arg(prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn rawk");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(stdin)
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait rawk");
    out.stdout
}

/// Due chiavi che differiscono solo per byte > 0x7f devono restare distinte
/// (con lossy collassano entrambe a U+FFFD e il count diventa 1 invece di 2).
#[test]
fn distinct_high_byte_keys_do_not_collide() {
    let prog = r#"{ a[$1]++ } END { n = 0; for (k in a) n++; print n }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"\xC3line1\n");
    input.extend_from_slice(b"\xE9line2\n");
    let out = run_with_stdin(prog, &input);
    let out_str = String::from_utf8_lossy(&out);
    assert_eq!(
        out_str.trim(),
        "2",
        "chiavi con byte alti diversi devono restare distinte; output ricevuto: {:?}",
        out_str
    );
}

/// Chiave con NUL embedded: deve essere indicizzabile senza troncamenti.
#[test]
fn nul_embedded_key_round_trips() {
    let prog = r#"BEGIN { SUBSEP = "\0"; a["x","y"] = "found"; print a["x","y"] }"#;
    let out = run_with_stdin(prog, b"");
    let out_str = String::from_utf8_lossy(&out);
    assert_eq!(out_str.trim(), "found");
}

/// for-in deve iterare esattamente sulle chiavi distinte byte-clean storate
/// (no collapsing su lossy conversion in storage). Le chiavi hanno suffisso
/// ASCII per restare distinguibili anche dopo il BRIDGE 7.2→7.5
/// (`update_record` lossy in field splitting), fuori scope 7.3.
/// Pre-7.3: count = 1 (collision); post-7.3: count = 2.
#[test]
fn for_in_returns_all_distinct_byte_keys() {
    let prog = r#"{ a[$1] = 1 } END { n = 0; for (k in a) n++; print n }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"\xC3suffix1\n");
    input.extend_from_slice(b"\xE9suffix2\n");
    input.extend_from_slice(b"\xC3suffix1\n");
    let out = run_with_stdin(prog, &input);
    let out_str = String::from_utf8_lossy(&out);
    assert_eq!(
        out_str.trim(),
        "2",
        "for-in deve enumerare 2 chiavi distinte; output: {:?}",
        out_str
    );
}
