mod common;
use std::process::Command;
use std::time::Duration;

#[test]
fn short_circuit_preserves_side_effects() {
    assert_eq!(
        common::run_with_stdin("BEGIN{x=0; print (0 && ++x),x; print (1 || ++x),x}", b""),
        b"0 0\n1 0\n"
    );
}

#[test]
fn exit_executes_end_and_interrupts_expressions() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rawk"));
    cmd.arg("function stop(){exit 7} BEGIN{print stop(),\"BAD\"} END{print \"END\"}");
    let out = rawk::test_support::run(cmd, b"", Duration::from_secs(2)).unwrap();
    assert_eq!(out.stdout, b"END\n");
    assert_eq!(out.code, Some(7));
}

#[test]
fn return_escapes_all_loop_forms() {
    assert_eq!(
        common::run_with_stdin(
            "function f(i){while(i++<2){return 7} return 9} function g(i){do{return 8}while(i++<2); return 9} BEGIN{print f(0),g(0)}",
            b""
        ),
        b"7 8\n"
    );
}
