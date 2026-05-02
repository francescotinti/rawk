use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_basic_print() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("{ print $1, $2 }")
        .write_stdin("hello world")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello world"));
}

#[test]
fn test_math_functions() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { print sqrt(16), int(3.9) }")
        .assert()
        .success()
        .stdout(predicate::str::contains("4 3"));
}

#[test]
fn test_arrays_and_delete() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { a[\"key\"] = 42; print a[\"key\"]; delete a[\"key\"]; print a[\"key\"] }")
        .assert()
        .success()
        .stdout(predicate::str::contains("42\n")); // Second print should be empty/uninitialized
}

#[test]
fn test_record_separator() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { RS = \"|\" } { print $1 }")
        .write_stdin("A|B|C")
        .assert()
        .success()
        .stdout("A\nB\nC\n");
}

#[test]
fn test_user_functions() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("function add(x, y) { return x + y } BEGIN { print add(5, 7) }")
        .assert()
        .success()
        .stdout(predicate::str::contains("12"));
}

#[test]
fn test_bitwise_and_time() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { print and(5, 3), lshift(1, 2) }")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 4"));
}
