mod common;

fn run(prog: &str) -> String {
    String::from_utf8(common::run_with_stdin(prog, b"")).unwrap()
}

#[test]
fn d_padding() {
    assert_eq!(run(r#"BEGIN{printf "%05d",42}"#), "00042");
}

#[test]
fn f_precision() {
    assert_eq!(run(r#"BEGIN{printf "%.2f",3.14159}"#), "3.14");
}

#[test]
fn s_truncate() {
    assert_eq!(run(r#"BEGIN{printf "%.3s","hello"}"#), "hel");
}

#[test]
fn x_hex() {
    assert_eq!(run(r#"BEGIN{printf "%x",255}"#), "ff");
}

#[test]
fn e_scientific() {
    let s = run(r#"BEGIN{printf "%e",1234.5}"#);
    assert!(s.starts_with("1.234500e"), "got {s}");
}
