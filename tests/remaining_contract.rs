use rawk::test_support as harness;
use std::{path::Path, process::Command, time::Duration};

fn compare(program: &str, input: &[u8], posix: bool) {
    let run = |binary: &Path| {
        let mut cmd = Command::new(binary);
        cmd.arg(program).env_remove("POSIXLY_CORRECT");
        if posix {
            cmd.env("POSIXLY_CORRECT", "1");
        }
        harness::run(cmd, input, Duration::from_secs(3)).unwrap()
    };
    let expected = run(&harness::reference_binary().unwrap());
    assert_eq!(
        expected.code,
        Some(0),
        "invalid oracle case: {program}: {expected:?}"
    );
    assert!(!expected.timed_out);
    assert_eq!(
        run(Path::new(env!("CARGO_BIN_EXE_rawk"))),
        expected,
        "{program}, input={input:?}, posix={posix}"
    );
}

#[test]
fn missing_fields_are_empty_strings_not_uninitialized_variables() {
    compare(
        r#"BEGIN {FS=":"} { print NF, ($5==0), ($5==""), (x==0), (x==""), ($1==0); y=$5; print (y==0), (y==""); $7="v";print ($5==0),($5=="");NF=1;print ($5==0),($5=="") }"#,
        b"\na::b\n0::\n",
        false,
    );
}

#[test]
fn replacement_escapes_have_separate_lexical_and_runtime_stages() {
    for posix in [false, true] {
        for count in 0..=8 {
            for suffix in ["&", "x", ""] {
                let replacement = format!("{}{suffix}", "\\".repeat(count));
                let literal =
                    format!(r#"BEGIN {{s="aba"; print gsub(/a/, "{replacement}", s),s}}"#);
                // Odd trailing slashes escape the closing quote. Unknown lexical escapes
                // retain the declared rawk extension; runtime escapes are compared below.
                if suffix == "&" || count % 2 == 0 {
                    compare(&literal, b"", posix);
                }
                compare(
                    r#"{s="aba";print gsub(/a/,$0,s),s;s="aba";print sub(/a/,$0,s),s}"#,
                    format!("{replacement}\n").as_bytes(),
                    posix,
                );
            }
        }
    }
}

#[test]
fn descending_bracket_ranges_follow_original_c() {
    for pattern in [
        "[z-a]",
        "[^z-a]",
        "[az-aQ]",
        "[a-z-a]",
        "[-a]",
        "[a-]",
        "[a\\-z]",
        "[]z-a]",
        "[[:digit:]z-a]",
        "[A-Zz-a]",
        "[a-b-c]",
        "[U-S]",
        "[\\x7a-\\x61]",
    ] {
        compare(
            &format!(
                r#"{{s=$0;print match(s,/{pattern}/),RSTART,RLENGTH;print gsub(/{pattern}/,"!",s),s;print split($0,a,/{pattern}/)}}"#
            ),
            b"a-bcQSUz019]\n\nxyz\n",
            false,
        );
    }
}

#[test]
fn numeric_rounding_uses_the_exact_binary_value() {
    for value in [
        "10409/8",
        "1.125",
        "1.375",
        "-1.125",
        "0.0000999999",
        "999999.5",
        "9.9995",
        "1e-8",
        "1e20",
        "0",
        "-0.0",
        r#""-0""#,
    ] {
        for spec in [
            "%.6g", "%.5g", "%.0g", "%.2f", "%+.2f", "%010.2f", "%-12.3e", "%#12.4G", "%.0e",
            "%.8E",
        ] {
            compare(
                &format!(
                    r#"BEGIN {{x={value};printf "{spec}\n",x;OFMT="{spec}";CONVFMT=OFMT; print x; printf "%s\n", x}}"#
                ),
                b"",
                false,
            );
        }
    }
}

#[test]
fn bracket_normalization_preserves_the_binary_profile() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rawk"));
    cmd.arg(r#"{s=$0;print gsub(/[\x00-\xff]/, "!", s),s;s=$0;print gsub(/[\x80-\xff]/, "!", s),s;s=$0;print gsub(/[[:cntrl:]]/, "!", s),s}"#);
    let out = harness::run(cmd, b"a\x00\x80\xff\n", Duration::from_secs(3)).unwrap();
    assert_eq!(out.code, Some(0));
    assert!(out.stderr.is_empty() && !out.timed_out);
    assert_eq!(out.stdout, b"4 !!!!\n2 a\x00!!\n1 a!\x80\xff\n");
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 64,
        rng_seed: proptest::test_runner::RngSeed::Fixed(20260920),
        ..proptest::test_runner::Config::default()
    })]
    #[test]
    fn generated_rational_values_and_float_formats(
        numerator in -10_000_000i32..10_000_000,
        denominator in 1u32..1_000_000,
        precision in 0usize..11,
        conversion in 0usize..5,
        flags in 0usize..5,
    ) {
        let conversion = ["e", "E", "f", "g", "G"][conversion];
        let flags = ["", "+", "-", "#", "0"][flags];
        let spec = format!("%{flags}12.{precision}{conversion}");
        compare(&format!(r#"BEGIN {{x={numerator}/{denominator}; printf "{spec}\n",x;CONVFMT="{spec}";print x ""}}"#), b"", false);
    }
}

#[test]
fn character_format_distinguishes_numeric_fields_from_literal_strings() {
    compare(
        r#"{printf "|%c|%4c|%-4c|%04c|%.2c|\n",$1,$1,$1,$1,$1}"#,
        b"65\n-1\n233\n17379\n",
        false,
    );
    compare(
        r#"BEGIN {printf "|%c|%4c|%.2c|\n","65","ab","z"}"#,
        b"",
        false,
    );
}
