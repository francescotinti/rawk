//! Phase 7.5 acceptance — printf/sprintf + field splitting byte-aware.
//!
//! RED prima della migrazione 7.5: `awk_sprintf` lavora su `&str`, `%c`
//! collassa byte alti su U+FFFD, `%s` perde byte non-UTF8, format string
//! con byte alti viene riscritta lossy, field splitting con FS byte alto
//! collide via lossy. GREEN dopo 7.5: parser byte-by-byte, `%c` emette
//! primo byte raw, `%s` emette bytes raw, FS byte-aware.

mod common;
use common::run_with_stdin;

/// printf "%s\n" con byte alti deve emettere i byte raw, non U+FFFD UTF-8.
#[test]
fn printf_s_preserves_high_byte() {
    let prog = r#"{ printf "%s\n", $0 }"#;
    let input = b"\xC3foo";
    let out = run_with_stdin(prog, input);
    assert_eq!(
        out, b"\xC3foo\n",
        "printf %s deve emettere bytes raw; output: {:?}",
        out
    );
}

/// printf "%c" deve emettere il PRIMO BYTE dell'argomento stringa, non
/// `chars().next()` (che collassa 0xC3 su U+FFFD = 3 byte UTF-8 EF BF BD).
#[test]
fn printf_c_emits_first_byte_raw() {
    let prog = r#"BEGIN { printf "%c", "\xC3X" }"#;
    let out = run_with_stdin(prog, b"");
    assert_eq!(
        out, b"\xC3",
        "printf %c deve emettere il primo byte raw 0xC3; output: {:?}",
        out
    );
}

/// Format string letterale contiene byte alti: devono passare integri
/// nell'output (pre-7.5 vengono riscritti lossy in `String::from_utf8_lossy`).
#[test]
fn printf_format_string_with_high_bytes() {
    let prog = r#"BEGIN { printf "\xC3%s\n", "x" }"#;
    let out = run_with_stdin(prog, b"");
    assert_eq!(
        out, b"\xC3x\n",
        "byte alti nel format letterale devono restare raw; output: {:?}",
        out
    );
}

/// sprintf con argomento byte-alto round-trippa byte-clean.
#[test]
fn sprintf_high_byte_round_trip() {
    let prog = r#"BEGIN { s = sprintf("%s", "\xC3foo"); printf "%s", s }"#;
    let out = run_with_stdin(prog, b"");
    assert_eq!(
        out, b"\xC3foo",
        "sprintf %s deve mantenere bytes raw; output: {:?}",
        out
    );
}

/// `print` con OFS byte alto deve produrre output con il byte separatore
/// integro (pre-7.5 il path `as_string_convfmt(&str)` riscrive OFS lossy).
#[test]
fn print_with_high_byte_ofs() {
    let prog = r#"BEGIN { OFS = "\xC3" } { print $1, $2 }"#;
    let out = run_with_stdin(prog, b"aa bb\n");
    assert_eq!(
        out, b"aa\xC3bb\n",
        "OFS byte-alto deve emergere raw in print; output: {:?}",
        out
    );
}

/// Field splitting con FS byte alto deve dividere i campi sul byte raw,
/// non sulla rappresentazione UTF-8 lossy (che collassa 0xC3 su U+FFFD).
/// Input `aaa\xC3bbb\xC3ccc` con `FS = "\xC3"` → NF=3, $1=aaa, $2=bbb, $3=ccc.
///
/// Nota: pre-7.5 questo testcase passa per **coincidenza** (la doppia lossy
/// sulla `line` e sul `fs` collassa entrambi a `\u{FFFD}` e split produce
/// 3 segmenti). Post-7.5 il path è byte-clean by-design. Manteniamo il
/// test come safety net contro regressioni in `update_record`.
#[test]
fn field_split_with_high_byte_fs() {
    let prog = r#"BEGIN { FS = "\xC3" } { print NF, $1, $2, $3 }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"aaa\xC3bbb\xC3ccc\n");
    let out = run_with_stdin(prog, &input);
    // I valori sono tutti ASCII: confronto in lossy testuale.
    let out_str = String::from_utf8_lossy(&out);
    assert_eq!(
        out_str.trim(),
        "3 aaa bbb ccc",
        "FS byte-alto deve splittare in 3 campi; output: {:?}",
        out_str
    );
}
