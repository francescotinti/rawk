use rawk::test_support as h;
use std::{path::Path, process::Command, time::Duration};
fn compare(program: &str, input: &[u8]) {
    let run = |bin: &Path| {
        let mut cmd = Command::new(bin);
        cmd.arg(program);
        h::run(cmd, input, Duration::from_secs(3)).unwrap()
    };
    let expected = run(&h::reference_binary().unwrap());
    assert_eq!(expected.code, Some(0));
    assert_eq!(
        run(Path::new(env!("CARGO_BIN_EXE_rawk"))),
        expected,
        "{program}"
    );
}
#[test]
fn dynamic_width_precision_and_consumption() {
    for width in [-9, 0, 9] {
        for precision in [-3, 0, 3] {
            for conv in ["d", "f", "g", "e", "s", "a"] {
                let value = if conv == "s" { "\"abcd\"" } else { "1.25" };
                compare(
                    &format!(
                        "BEGIN{{printf \"[%*.*{conv}] %d\\n\",{width},{precision},{value},73;print sprintf(\"[%*.*{conv}]\",{width},{precision},{value})}}"
                    ),
                    b"",
                );
            }
        }
    }
    compare("BEGIN{i=4;printf \"[%*.*f] %d\\n\",i++,i++,1.25,i}", b"");
}
#[test]
fn string_and_literal_range_errors_do_not_discard_arithmetic_subnormals() {
    compare(
        "BEGIN {printf \"%.17g %.17g %.17g %.17g\\n\",1e-308,1e999,0e-999,1e-300/1e8;print (\"1e-308\"+0),(\"1e999\"+0)} {print ($1==0),($1+0),$1}",
        b"1e-308\n-1e-999\n0e-999\n1e999\n2.2250738585072014e-308\n",
    );
}
#[test]
fn missing_star_arguments_are_errors() {
    for program in ["BEGIN{printf \"%*d\"}", "BEGIN{print sprintf(\"%*.*f\",2)}"] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_rawk"));
        cmd.arg(program);
        let out = h::run(cmd, b"", Duration::from_secs(3)).unwrap();
        assert_eq!(out.code, Some(2));
        assert!(!out.stderr.is_empty());
    }
}

#[test]
fn integer_flags_precision_and_signs() {
    for conversion in ['d', 'i', 'u', 'o', 'x', 'X'] {
        for flags in ["", "-", "0", "#", "+", " ", "-0", "+0", "#0"] {
            for precision in ["", ".0", ".4"] {
                let format = format!("{flags}9{precision}{conversion}");
                compare(
                    &format!("BEGIN {{ for(i=-3;i<=3;i++) printf \"[%{format}]\\n\",i }}"),
                    b"",
                );
            }
        }
    }
}
