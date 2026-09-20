use rawk::test_support as h;
use std::{process::Command, time::Duration};

fn run(binary: &std::path::Path, program: &str, locale: &str) -> h::Outcome {
    let mut command = Command::new(binary);
    command.env("LC_ALL", locale).arg(program);
    h::run(command, b"", Duration::from_secs(5)).unwrap()
}
fn compare(program: &str, locale: &str) {
    let c = run(&h::reference_binary().unwrap(), program, locale);
    let rust = run(
        std::path::Path::new(env!("CARGO_BIN_EXE_rawk")),
        program,
        locale,
    );
    assert!(
        !c.timed_out && c.code == Some(0) && c.stderr.is_empty(),
        "{c:?}"
    );
    assert_eq!(rust, c, "locale={locale}, program={program}");
}
const LOCALES: [&str; 2] = ["en_US.ISO8859-1", "tr_TR.ISO8859-9"];

#[test]
fn required_single_byte_locales_are_active() {
    for locale in LOCALES {
        for binary in [
            h::reference_binary().unwrap(),
            env!("CARGO_BIN_EXE_rawk").into(),
        ] {
            let out = run(
                &binary,
                r#"BEGIN{s=sprintf("%c",233);print length(s),toupper(s),(s~/[[:alpha:]]/)}"#,
                locale,
            );
            assert!(
                !out.timed_out && out.code == Some(0) && out.stderr.is_empty(),
                "{out:?}"
            );
            assert_eq!(
                out.stdout,
                b"1 \xc9 1\n",
                "{locale} in {}",
                binary.display()
            );
        }
    }
}

#[test]
fn every_nonzero_byte_case_conversion_matches_c() {
    for locale in LOCALES {
        compare(
            r#"BEGIN{for(i=1;i<256;i++){s=sprintf("%c",i);printf "%s%s",toupper(s),tolower(s)}}"#,
            locale,
        );
    }
}

#[test]
fn every_nonzero_byte_in_each_posix_class_matches_c() {
    for locale in LOCALES {
        for class in [
            "alnum", "alpha", "blank", "cntrl", "digit", "graph", "lower", "print", "punct",
            "space", "upper", "xdigit",
        ] {
            compare(
                &format!(
                    r#"BEGIN{{for(i=1;i<256;i++){{s=sprintf("%c",i);print i,(s~/[[:{class}:]]/),(s~/[^[:{class}:]]/),match(s,/[[:{class}:]]/)}}}}"#
                ),
                locale,
            );
        }
    }
}

#[test]
fn byte_positions_ranges_splits_and_substitution_match_c() {
    for locale in LOCALES {
        compare(
            r#"BEGIN{s=sprintf("%c%c%c",233,201,255);print length(s),index(s,sprintf("%c",201)),split(s,a,"");printf "[%3.1s][%c]\n",s,s;print substr(s,2,1);print gsub(/[[:alpha:]]/,"X",s),s; s=sprintf("%c%c",195,169);print length(s),split(s,a,"");print (s~/[\200-\377]/)}"#,
            locale,
        );
    }
}

#[test]
fn nul_preservation_remains_an_explicit_rust_extension() {
    for locale in LOCALES {
        let out = run(
            std::path::Path::new(env!("CARGO_BIN_EXE_rawk")),
            r#"BEGIN{s=sprintf("%c%c%c",233,0,201);printf "%s",toupper(s);printf "%s",tolower(s)}"#,
            locale,
        );
        assert!(
            !out.timed_out && out.code == Some(0) && out.stderr.is_empty(),
            "{out:?}"
        );
        assert_eq!(out.stdout, b"\xc9\0\xc9\xe9\0\xe9");
    }
}

#[test]
fn legacy_locale_selection_obeys_precedence_and_is_fixed_at_startup() {
    let program = r#"BEGIN{s=sprintf("%c",233);printf "%s",toupper(s);ENVIRON["LC_ALL"]="C";printf "%s",toupper(s)}"#;
    for (all, ctype, lang, expected) in [
        (Some("C"), "en_US.ISO8859-1", "C", &b"\xe9\xe9"[..]),
        (None, "en_US.ISO8859-1", "C", &b"\xc9\xc9"[..]),
        (Some(""), "", "tr_TR.ISO8859-9", &b"\xc9\xc9"[..]),
        (
            Some("nonexistent_RAWK_locale"),
            "en_US.ISO8859-1",
            "C",
            &b"\xe9\xe9"[..],
        ),
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
                .arg(program);
            if let Some(all) = all {
                command.env("LC_ALL", all);
            }
            let out = h::run(command, b"", Duration::from_secs(5)).unwrap();
            assert!(
                !out.timed_out && out.code == Some(0) && out.stderr.is_empty(),
                "{out:?}"
            );
            assert_eq!(
                out.stdout,
                expected,
                "{} env={all:?}/{ctype}/{lang}",
                binary.display()
            );
        }
    }
}
