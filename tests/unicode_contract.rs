use rawk::test_support as h;
use std::{path::Path, process::Command, time::Duration};

#[test]
fn required_locales_are_active_in_both_interpreters() {
    // A differential test alone can pass if both programs silently fall back
    // to C. Require observable UTF-8 and non-ASCII case mapping independently.
    for binary in [
        h::reference_binary().unwrap(),
        env!("CARGO_BIN_EXE_rawk").into(),
    ] {
        for (locale, expected) in [
            ("C", "4 é\n"),
            ("en_US.UTF-8", "1 É\n"),
            ("tr_TR.UTF-8", "1 É\n"),
        ] {
            let mut command = Command::new(&binary);
            command
                .env("LC_ALL", locale)
                .arg(r#"BEGIN{print length("😀"),toupper("é")}"#);
            let out = h::run(command, b"", Duration::from_secs(5)).unwrap();
            assert!(!out.timed_out && out.code == Some(0), "{out:?}");
            assert!(out.stderr.is_empty(), "{out:?}");
            assert_eq!(
                out.stdout,
                expected.as_bytes(),
                "required locale {locale} unavailable or incorrect in {}",
                binary.display()
            );
        }
    }
}

fn compare(program: &str, input: &[u8], env: &[(&str, &str)]) {
    let run = |binary| {
        let mut command = Command::new(binary);
        command
            .arg(program)
            .env_remove("LC_ALL")
            .env_remove("LC_CTYPE")
            .env_remove("LANG");
        command.envs(env.iter().copied());
        h::run(command, input, Duration::from_secs(5)).unwrap()
    };
    let c = run(h::reference_binary().unwrap());
    let rust = run(env!("CARGO_BIN_EXE_rawk").into());
    assert!(!c.timed_out && !rust.timed_out);
    assert_eq!(rust, c, "{program} env={env:?} input={input:?}");
}

#[test]
fn locale_precedence_and_character_operations() {
    for env in [
        vec![("LC_ALL", "C"), ("LC_CTYPE", "en_US.UTF-8")],
        vec![("LANG", "en_US.UTF-8")],
        vec![("LC_ALL", ""), ("LC_CTYPE", "en_US.UTF-8"), ("LANG", "C")],
        vec![("LC_CTYPE", "C"), ("LANG", "en_US.UTF-8")],
        vec![
            ("LC_ALL", "nonexistent_RAWK_locale"),
            ("LANG", "en_US.UTF-8"),
        ],
    ] {
        compare(
            r#"BEGIN{s="aé😀z";print length(s),substr(s,2,2),index(s,"z"); match(s,/z/);print RSTART,RLENGTH;print split(s,a,""); ENVIRON["LC_ALL"]="C";print length(s)}"#,
            b"",
            &env,
        );
    }
    for locale in ["C", "en_US.UTF-8", "tr_TR.UTF-8"] {
        compare(
            r#"BEGIN{print toupper("éßıi"),tolower("ÉİIΣ"); print 1.5,"1,5"+0; printf "%.2f\n",1.5}"#,
            b"",
            &[("LC_ALL", locale)],
        );
    }
}

#[test]
fn utf8_regex_empty_matches_and_mixed_bytes() {
    for program in [
        r#"{print length($0),substr($0,2,2),index($0,"z");match($0,/z/);print RSTART,RLENGTH}"#,
        r#"{s=$0;print gsub(//,"X",s),s}"#,
        r#"{s=$0;print gsub(/.*/,"X",s),s}"#,
        r#"{print $0 ~ /^.$/,$0 ~ /^..$/;print split($0,a,"")}"#,
        r#"{match($0,/é|é😀/);print RSTART,RLENGTH}"#,
    ] {
        compare(
            program,
            b"a\xc3\xa9\xf0\x9f\x98\x80z\n\xffz\n\xc3z\n\xc0\xafz\n\xf0\x9fz\n",
            &[("LC_ALL", "en_US.UTF-8")],
        );
    }
}

#[test]
fn unicode_formatting_and_source_escapes() {
    for locale in ["C", "en_US.UTF-8"] {
        compare(
            r#"BEGIN{s="é😀";for(i=0;i<4;i++)printf "[%5.*s][%-5.*s][%05.*s]\n",i,s,i,s,i,s; printf "[%4c][%-4c][%.0c]\n",s,233,s;print "\u03bb",length("\u03bb"),"\u1F600"}"#,
            b"",
            &[("LC_ALL", locale)],
        );
    }
}

#[test]
fn unicode_classes_ranges_and_anchors() {
    for re in [
        "[α-ω]",
        "[^α-ω]",
        "[[:alpha:]]",
        "[[:upper:]]",
        "[ω-α]",
        "[a-z-a]",
        "[^a-z-a]",
        "(é|é😀)+",
        "^😀.$",
        r"\u03bb",
        r"\351",
    ] {
        let program = format!("{{match($0,/{re}/);print RSTART,RLENGTH}}");
        compare(
            &program,
            "λ\né\nÉ\n😀λ\na\n-\nz\nzzé😀\n".as_bytes(),
            &[("LC_ALL", "en_US.UTF-8")],
        );
    }
}

#[test]
fn utf8_record_separators_across_buffers() {
    // An incomplete euro at an 8192-byte read boundary must not match â (the
    // first raw byte 0xe2 decoded in isolation). EOF still admits invalid bytes.
    compare(
        r#"BEGIN{RS="â"}{print NR,length($0)}"#,
        "€".repeat(3000).as_bytes(),
        &[("LC_ALL", "en_US.UTF-8")],
    );
    compare(
        r#"BEGIN{RS="â"}{print NR,length($0)}"#,
        b"a\xe2\x82",
        &[("LC_ALL", "en_US.UTF-8")],
    );
    let data = format!("{}😀{}😀z", "é".repeat(4095), "λ".repeat(4097));
    compare(
        r#"BEGIN{RS="😀+"}{print NR,length($0),substr($0,1,1)}"#,
        data.as_bytes(),
        &[("LC_ALL", "en_US.UTF-8")],
    );
    compare(
        r#"BEGIN{RS="^é"}{print NR,length($0)}"#,
        data.as_bytes(),
        &[("LC_ALL", "en_US.UTF-8")],
    );
}

#[test]
fn original_utf8_drivers() {
    let output = tempfile::NamedTempFile::new().unwrap();
    let mut command = Command::new("python3");
    command
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/full_driver_audit.py"))
        .args([
            "--rawk",
            env!("CARGO_BIN_EXE_rawk"),
            "--drivers",
            "T.utf",
            "T.utfre",
            "--locale",
            "en_US.UTF-8",
            "--check",
            "--output",
        ])
        .arg(output.path());
    let result = h::run(command, b"", Duration::from_secs(90)).unwrap();
    assert_eq!(result.code, Some(0), "{result:?}");
    assert!(!result.timed_out && result.stderr.is_empty());
}

#[test]
fn binary_nul_extension_and_case_conversion_errors() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rawk"));
    command
        .env("LC_ALL", "en_US.UTF-8")
        .arg(r#"BEGIN{s="é\0😀";print length(s);print gsub(/./,"x",s),s;print toupper("é\0ß")}"#);
    let out = h::run(command, b"", Duration::from_secs(5)).unwrap();
    assert_eq!(out.code, Some(0), "{out:?}");
    assert_eq!(out.stdout, "3\n3 xxx\nÉ\0ß\n".as_bytes());
    let mut command = Command::new(env!("CARGO_BIN_EXE_rawk"));
    command
        .env("LC_ALL", "en_US.UTF-8")
        .arg(r#"BEGIN{print toupper("\xff")}"#);
    let out = h::run(command, b"", Duration::from_secs(5)).unwrap();
    assert_eq!(out.code, Some(2));
    assert_eq!(
        out.stderr,
        b"rawk: illegal byte sequence in case conversion\n"
    );
}

#[test]
fn repeated_matches_and_index_inside_a_multibyte_unit() {
    compare(
        r#"BEGIN{print index("é","\xa9"),index("😀","\x98"),index("","")}"#,
        b"",
        &[("LC_ALL", "en_US.UTF-8")],
    );
    compare(
        r#"{print gsub(/./,"x"),length($0)}"#,
        "é😀".repeat(5000).as_bytes(),
        &[("LC_ALL", "en_US.UTF-8")],
    );
}

#[test]
fn boolean_regex_search_preserves_rune_alignment_and_empty_matches() {
    // Encoded Ā followed by another rune contains the key for 𐀀 starting
    // at byte 1. That unaligned hit must be rejected, and searching continued.
    for pattern in ["𐀀", "𐀀|𐀀a", "", "^", "$", "^𐀀", "é.*|é", ".", "[^é]"] {
        let program = format!(
            "{{print ($0 ~ /{pattern}/),($0 !~ /{pattern}/),(match($0,/{pattern}/)>0)}} /{pattern}/ {{print \"rule\"}}"
        );
        compare(
            &program,
            b"\xc4\x80a\n\xc4\x80a\xf0\x90\x80\x80\n\n\xc3\xa9z\n\xff\n\xc3\n",
            &[("LC_ALL", "en_US.UTF-8")],
        );
    }
    // Embedded NUL is a deliberate Rust extension; boolean and positional
    // searches must agree without treating zero bytes in encoded keys as NUL.
    let mut command = Command::new(env!("CARGO_BIN_EXE_rawk"));
    command.env("LC_ALL", "en_US.UTF-8").arg(
        r#"BEGIN{s="Āa";print (s~/\0/),(s!~/\0/),(match(s,/\0/)>0);s="Ā\0a";print (s~/\0/),(s!~/\0/),(match(s,/\0/)>0)}"#,
    );
    let out = h::run(command, b"", Duration::from_secs(5)).unwrap();
    assert_eq!(out.code, Some(0));
    assert!(!out.timed_out && out.stderr.is_empty(), "{out:?}");
    assert_eq!(out.stdout, b"0 1 0\n1 0 1\n");
}
