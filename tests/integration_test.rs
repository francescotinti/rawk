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

#[test]
fn test_regex_matching() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { match(\"hello 123 world\", \"[0-9]+\"); print RSTART, RLENGTH }")
        .assert()
        .success()
        .stdout(predicate::str::contains("7 3"));
}

#[test]
fn test_string_manipulation() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { s = \"foo bar\"; gsub(\"bar\", \"baz\", s); print s, length(s), toupper(s) }")
        .assert()
        .success()
        .stdout(predicate::str::contains("foo baz 7 FOO BAZ"));
}

#[test]
fn test_control_flow() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { sum = 0; i = 1; while(i<=5) { sum = sum + i; i = i + 1 }; print sum }")
        .assert()
        .success()
        .stdout(predicate::str::contains("15"));
}

#[test]
fn test_type_coercion() {
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { print \"10\" + 5, \"42\" == 42, \"abc\" + 0 }")
        .assert()
        .success()
        .stdout(predicate::str::contains("15 1 0"));
}

// --- Official GNU Awk Manual Tests ---
// These examples are directly sourced from the official GNU Awk User's Guide
// Source: https://www.gnu.org/software/gawk/manual/html_node/Very-Simple.html

#[test]
fn test_gawk_manual_longest_line() {
    // "Print the length of the longest input line."
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("{ if (length($0) > max) max = length($0) } END { print max }")
        .write_stdin("short\nthis is a much longer line\ntiny")
        .assert()
        .success()
        .stdout(predicate::str::contains("26"));
}

#[test]
fn test_gawk_manual_seven_random_numbers() {
    // "Print seven random numbers from 0 to 100."
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("BEGIN { for (i = 1; i <= 7; i++) print int(101 * rand()) }")
        .assert()
        .success()
        .stdout(predicate::function(|s: &str| {
            // It should output exactly 7 lines, each containing a number
            let lines: Vec<&str> = s.trim().split('\n').collect();
            lines.len() == 7 && lines.iter().all(|l| l.parse::<i32>().is_ok())
        }));
}

// Source: https://www.gnu.org/software/gawk/manual/html_node/Two-Rules.html
#[test]
fn test_gawk_manual_two_rules() {
    // "An Example with Two Rules"
    let mut cmd = Command::cargo_bin("rawk").unwrap();
    cmd.arg("/12/ { print $0 } /21/ { print $0 }")
        .write_stdin("foo 12 bar\nbaz 21 qux\nno match")
        .assert()
        .success()
        .stdout(predicate::str::contains("foo 12 bar\nbaz 21 qux"));
}
