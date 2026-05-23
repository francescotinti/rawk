//! Phase 7.4 acceptance — regex matcher byte-aware.
//!
//! RED prima della migrazione 7.4 (lossy conversion sui subject e sui pattern
//! collassa byte > 0x7f in U+FFFD: `\xC3` e `\xE9` matchano lo stesso regex,
//! RSTART/RLENGTH sono in offset lossy e non in byte). GREEN dopo 7.4.2:
//! `regex_cache: HashMap<Vec<u8>, regex::bytes::Regex>` + match su `&[u8]`.

mod common;
use common::run_with_stdin;

/// `match()` deve restituire RSTART in byte, non in unità lossy.
/// Input `\xC3\xC3foo`: `foo` inizia al byte 3 (1-based).
/// Pre-7.4: con lossy `\xC3` diventa U+FFFD (3 byte UTF-8) → RSTART sbagliato.
#[test]
fn match_builtin_high_byte_pattern_position() {
    let prog = r#"{ if (match($0, /foo/)) print RSTART }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"\xC3\xC3foo\n");
    let out = run_with_stdin(prog, &input);
    let out_str = String::from_utf8_lossy(&out);
    assert_eq!(
        out_str.trim(),
        "3",
        "RSTART deve essere byte-offset 1-based; output: {:?}",
        out_str
    );
}

/// Due record con byte alti differenti: regex `/\xC3/` deve matchare solo
/// quello con `\xC3`. Pre-7.4: lossy collassa entrambi a U+FFFD → conteggio
/// errato (0 oppure 2 invece di 1).
#[test]
fn match_distinguishes_high_bytes() {
    let prog = r#"/\xC3/ { c++ } END { print c+0 }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"\xC3X\n");
    input.extend_from_slice(b"\xE9X\n");
    let out = run_with_stdin(prog, &input);
    let out_str = String::from_utf8_lossy(&out);
    assert_eq!(
        out_str.trim(),
        "1",
        "regex /\\xC3/ deve matchare solo il record con byte 0xC3; output: {:?}",
        out_str
    );
}

/// `sub()` su sequenza di byte alti: il pattern `/\xC3+/` deve coprire la run
/// di byte 0xC3 e sostituirla, lasciando il resto byte-pulito.
#[test]
fn sub_replaces_high_byte_run() {
    let prog = r#"{ sub(/\xC3+/, "Z"); print }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"aa\xC3\xC3bb\n");
    let out = run_with_stdin(prog, &input);
    assert_eq!(
        out, b"aaZbb\n",
        "sub deve sostituire la run \\xC3+ con Z byte-pulito; output: {:?}",
        out
    );
}

/// POSIX AWK: `&` nella replacement è il whole match. `\&` è il letterale.
/// `gsub(/x/, "[&]")` su "axbxc" deve dare "a[x]b[x]c".
/// Pre-7.4: `regex::Regex::replace_all` tratta `&` come letterale (non $0)
/// → output errato `a[&]b[&]c`. Bug latente fixato in 7.4 via closure Replacer.
#[test]
fn gsub_ampersand_includes_whole_match() {
    let prog = r#"{ gsub(/x/, "[&]"); print }"#;
    let out = run_with_stdin(prog, b"axbxc\n");
    assert_eq!(
        out, b"a[x]b[x]c\n",
        "& in replacement deve essere il whole match POSIX; output: {:?}",
        out
    );
}

/// RS-regex con byte alti: `RS = "\xC3+"` deve splittare il flusso in record
/// sui run di 0xC3, indipendentemente dalla validità UTF-8.
/// Tre record attesi: "aaa", "bbb", "ccc".
#[test]
fn rs_regex_high_byte_separator_splits_correctly() {
    let prog = r#"BEGIN { RS = "\xC3+" } { print NR, $0 }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"aaa\xC3\xC3bbb\xC3ccc");
    let out = run_with_stdin(prog, &input);
    let out_str = String::from_utf8_lossy(&out);
    assert_eq!(
        out_str.trim(),
        "1 aaa\n2 bbb\n3 ccc",
        "RS regex su byte alti deve produrre 3 record; output: {:?}",
        out_str
    );
}

/// Subject con NUL embedded sul path buffered (RS-regex): il match avviene
/// sul pattern `\xC3+`, NUL fra i record è preservato dentro `$0`.
/// Pre-7.4: `read_to_string` può rifiutare/troncare (NUL ok in UTF-8 ma path
/// `&str` non garantisce byte-exact slicing su sequenze invalide accumulate).
#[test]
fn nul_inside_subject_with_rs_regex() {
    let prog = r#"BEGIN { RS = "\xC3+" } { print NR, length($0) }"#;
    let mut input: Vec<u8> = Vec::new();
    input.extend_from_slice(b"aaa\xC3bbb\x00ccc\xC3ddd");
    let out = run_with_stdin(prog, &input);
    let out_str = String::from_utf8_lossy(&out);
    // Tre record: "aaa" (3), "bbb\x00ccc" (7), "ddd" (3). length() è byte-count POSIX.
    assert_eq!(
        out_str.trim(),
        "1 3\n2 7\n3 3",
        "length() byte-count su record con NUL embedded; output: {:?}",
        out_str
    );
}
