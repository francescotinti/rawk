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

/// for-in deve restituire le chiavi byte-arbitrarie senza corromperle:
/// length() su ogni chiave deve dare 1 (POSIX byte-count).
#[test]
fn for_in_preserves_byte_keys_length() {
    let prog = r#"
        BEGIN { a["\xC3"] = 1; a["\xE9"] = 2 }
        END   { for (k in a) print length(k) }
    "#;
    let out = run_with_stdin(prog, b"");
    let out_str = String::from_utf8_lossy(&out);
    let lines: Vec<&str> = out_str.lines().collect();
    assert_eq!(lines.len(), 2, "atteso 2 chiavi distinte, got {:?}", lines);
    for line in &lines {
        assert_eq!(
            line.trim(),
            "1",
            "ogni chiave deve essere lunga 1 byte (POSIX length byte-count); got {:?}",
            line
        );
    }
}
