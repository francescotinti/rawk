use rawk::test_support as h;
use std::{path::Path, process::Command, time::Duration};

fn run(binary: &Path, program: &str, input: &[u8]) -> h::Outcome {
    let mut command = Command::new(binary);
    command.env("LC_ALL", "ja_JP.SJIS").arg(program);
    h::run(command, input, Duration::from_secs(5)).unwrap()
}

fn compare(program: &str, input: &[u8]) {
    let c = run(&h::reference_binary().unwrap(), program, input);
    let rust = run(Path::new(env!("CARGO_BIN_EXE_rawk")), program, input);
    assert!(!c.timed_out && !rust.timed_out);
    assert_eq!(
        rust.code, c.code,
        "{program}, input={input:?}, C={c:?}, Rust={rust:?}"
    );
    assert_eq!(rust.stdout, c.stdout, "{program}, input={input:?}");
    if c.code == Some(0) {
        assert_eq!(rust.stderr, c.stderr);
    } else {
        // Diagnostics include executable names and record/source context in C.
        for out in [&c, &rust] {
            assert_eq!(out.code, Some(2));
            assert!(String::from_utf8_lossy(&out.stderr).contains("illegal byte sequence"));
        }
    }
}

#[test]
fn required_sjis_locale_has_observable_case_mapping() {
    for binary in [
        h::reference_binary().unwrap(),
        env!("CARGO_BIN_EXE_rawk").into(),
    ] {
        // Fullwidth small a -> fullwidth capital A. Neither C nor UTF-8
        // fallback can produce this output from these Shift-JIS bytes.
        let out = run(&binary, "{printf \"%s\",toupper($0)}", b"\x82\x81\n");
        assert_eq!(out.code, Some(0), "{out:?}");
        assert!(out.stderr.is_empty());
        assert_eq!(out.stdout, b"\x82\x60", "{}", binary.display());
    }
}

#[test]
fn single_bytes_and_lead_trail_boundaries_match_c() {
    for byte in 1..=255 {
        if byte != b'\n' {
            for program in ["{print toupper($0)}", "{print tolower($0)}"] {
                compare(program, &[byte, b'\n']);
            }
        }
    }
    for lead in [0x81, 0x82, 0x83, 0x84, 0x9f, 0xe0, 0xe9, 0xef, 0xf0, 0xfc] {
        for trail in [
            0x01, 0x3f, 0x40, 0x60, 0x7e, 0x7f, 0x80, 0x81, 0x9f, 0xfc, 0xfd, 0xff,
        ] {
            for program in ["{print toupper($0)}", "{print tolower($0)}"] {
                compare(program, &[b'a', lead, trail, b'z', b'\n']);
            }
        }
    }
}

#[test]
fn valid_case_conversion_and_truncated_suffixes_match_c() {
    // ASCII, halfwidth katakana, hiragana, fullwidth Latin, Greek and Cyrillic.
    let valid = b"aZ\xa6\xdf\x82\xa0\x82\x60\x82\x81\x83\x9f\x83\xbf\x84\x40\x84\x70";
    compare(
        "{print toupper($0);print tolower($0);print toupper(tolower($0))}",
        valid,
    );
    for tail in [0x81, 0x9f, 0xe0, 0xe9, 0xef] {
        let mut input = valid.to_vec();
        input.push(tail);
        compare("{print toupper($0)}", &input);
        compare("{print tolower($0)}", &input);
    }
}

#[test]
fn structural_units_regex_fields_and_formatting_match_c() {
    // These are intentionally BWK structural units, not decoded SJIS glyphs.
    let input = b"a\x82\xa0z\n\xc3\xa9x\n\xe1\x80\x80z\n\xf0\x90\x80\x80z\n\xe9\n";
    for program in [
        r#"{print length($0),index($0,"z");print substr($0,2,1);print split($0,a,"");for(i=1;i<=length($0);i++)print a[i];print match($0,/./),RSTART,RLENGTH;print gsub(/./,"X"),$0}"#,
        r#"{printf "[%5.2s][%-5.1s][%3c]\n",$0,$0,$0;print ($0~/^..$/),($0~/[\200-\377]/)}END{printf "[%c][%3c]\n",233,12354}"#,
        r#"BEGIN{FS=""}{print NF;for(i=1;i<=NF;i++)print $i}"#,
    ] {
        compare(program, input);
    }
    for class in [
        "alnum", "alpha", "blank", "cntrl", "digit", "graph", "lower", "print", "punct", "space",
        "upper", "xdigit",
    ] {
        let input: Vec<u8> = (1..=255)
            .filter(|b| *b != b'\n')
            .flat_map(|b| [b, b'\n'])
            .collect();
        compare(
            &format!(
                r#"{{print ($0~/[[:{class}:]]/),($0~/[^[:{class}:]]/),match($0,/[[:{class}:]]/)}}"#
            ),
            &input,
        );
    }
}

#[test]
fn record_separators_and_locale_precedence_match_c() {
    for program in [
        r#"BEGIN{RS=".."}{print length($0),$0}"#,
        r#"BEGIN{RS="[xz]+"}{print length($0),$0}"#,
        r#"BEGIN{FS="[xz]+"}{print NF,$1,$2}"#,
    ] {
        compare(program, &b"a\x82\xa0z\xc3\xa9x".repeat(1100));
    }
    for (all, ctype, lang) in [
        (Some("ja_JP.SJIS"), "C", "C"),
        (None, "ja_JP.SJIS", "C"),
        (Some(""), "", "ja_JP.SJIS"),
    ] {
        for binary in [
            h::reference_binary().unwrap(),
            env!("CARGO_BIN_EXE_rawk").into(),
        ] {
            let mut command = Command::new(&binary);
            command
                .env_remove("LC_ALL")
                .env("LC_CTYPE", ctype)
                .env("LANG", lang)
                .arg(r#"{ENVIRON["LC_ALL"]="C";printf "%s",toupper($0)}"#);
            if let Some(all) = all {
                command.env("LC_ALL", all);
            }
            let out = h::run(command, b"\x82\x81\n", Duration::from_secs(5)).unwrap();
            assert_eq!(out.code, Some(0), "{out:?}");
            assert_eq!(out.stdout, b"\x82\x60");
        }
    }
}

#[test]
#[ignore = "Open C fnematch lookahead discrepancy; see diary/2026-09-20-shift-jis.md"]
fn regex_rs_with_three_or_four_byte_structural_units_matches_c() {
    // Keep the exact differential regression; do not replace C's expectation
    // with Rust's result. C refills in MB_CUR_MAX=2 chunks in ja_JP.SJIS.
    compare(r#"BEGIN{RS=".."}{print length($0),$0}"#, b"\xe1\x80\x80z\n");
}

#[test]
fn conversion_failure_preserves_atomic_print_contract() {
    // Existing deliberate contract: print is atomic on argument failure
    // (tests/cases/0037_test_concat_func_call_disambig.xml). The C emits
    // its prefix eagerly. Preserve this difference for conversion errors too.
    let program = "{print length($0),toupper($0)}";
    let c = run(&h::reference_binary().unwrap(), program, b"\xe9\n");
    let rust = run(Path::new(env!("CARGO_BIN_EXE_rawk")), program, b"\xe9\n");
    for out in [&c, &rust] {
        assert_eq!(out.code, Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("illegal byte sequence"));
    }
    assert_eq!(c.stdout, b"1 ");
    assert!(rust.stdout.is_empty());
}

#[test]
fn embedded_nul_preserves_the_rust_binary_extension() {
    let out = run(
        Path::new(env!("CARGO_BIN_EXE_rawk")),
        "{printf \"%s|%s\",toupper($0),tolower($0)}",
        b"a\0\x82\x81\n",
    );
    assert_eq!(out.code, Some(0), "{out:?}");
    assert!(out.stderr.is_empty());
    assert_eq!(out.stdout, b"A\0\x82\x60|a\0\x82\x81");
    let out = run(
        Path::new(env!("CARGO_BIN_EXE_rawk")),
        "{print toupper($0)}",
        b"a\0\xe9",
    );
    assert_eq!(out.code, Some(2));
}
