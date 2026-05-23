//! Helper condivisi per le integration test byte-aware (Phase 7).
//!
//! Estratto in 7.5 da `bytes_array_keys.rs` e `bytes_regex_match.rs`
//! per evitare un terzo copia-incolla in `bytes_printf.rs`.

use std::io::Write;
use std::process::{Command, Stdio};

const RAWK: &str = env!("CARGO_BIN_EXE_rawk");

/// Esegue rawk con `prog` come programma AWK e `stdin` come input grezzo.
/// Ritorna stdout grezzo (`Vec<u8>`) — il caller decide se confrontare in
/// bytes o convertire lossy per asserzioni testuali.
pub fn run_with_stdin(prog: &str, stdin: &[u8]) -> Vec<u8> {
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
