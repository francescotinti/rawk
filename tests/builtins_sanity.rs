mod common;

fn run(prog: &str) -> String {
    String::from_utf8(common::run_with_stdin(prog, b"")).unwrap()
}

#[test]
fn length_string() {
    assert_eq!(run("BEGIN{print length(\"hello\")}").trim(), "5");
}

#[test]
fn substr_basic() {
    assert_eq!(run("BEGIN{print substr(\"hello\",2,3)}").trim(), "ell");
}

#[test]
fn split_basic() {
    assert_eq!(
        run("BEGIN{n=split(\"a:b:c\",a,\":\"); print n,a[1],a[3]}").trim(),
        "3 a c"
    );
}

#[test]
fn sprintf_int() {
    assert_eq!(run("BEGIN{print sprintf(\"%05d\",42)}").trim(), "00042");
}

#[test]
fn match_basic() {
    assert_eq!(
        run("BEGIN{print match(\"hello\",/ll/),RSTART,RLENGTH}").trim(),
        "3 3 2"
    );
}
