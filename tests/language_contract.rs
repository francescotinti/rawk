mod common;

#[test]
fn numeric_and_string_values_keep_their_types() {
    assert_eq!(
        common::run_with_stdin(
            "BEGIN {print (\"01\"==\"1\"),(\"10\"<\"2\"),\"12x\"+0}",
            b""
        ),
        b"0 1 12\n"
    );
}
#[test]
fn expressions_and_escapes() {
    assert_eq!(
        common::run_with_stdin(
            r#"BEGIN{_x=.5; print "a\"b",_x,-2^2,(y=3); $0="3"; $1++; print $1}"#,
            b""
        ),
        b"a\"b 0.5 -4 3\n4\n"
    );
}
#[test]
fn range_patterns_and_empty_actions() {
    assert_eq!(
        common::run_with_stdin("BEGIN{} NR==1,NR==2 {print} END{}", b"a\nb\nc\n"),
        b"a\nb\n"
    );
}
#[test]
fn array_parameters_are_references() {
    assert_eq!(
        common::run_with_stdin("function f(a){a[1]=9} BEGIN{x[1]=1; f(x); print x[1]}", b""),
        b"9\n"
    );
}
#[test]
fn lvalue_indices_are_evaluated_once() {
    assert_eq!(
        common::run_with_stdin(
            "BEGIN{i=1; a[1]=3; a[i++]++; print i,a[1]; a[i++] += 5; print i,a[2]}",
            b""
        ),
        b"2 4\n3 5\n"
    );
}

#[test]
fn array_reads_create_entries() {
    assert_eq!(
        common::run_with_stdin("BEGIN{print a[1]; print (1 in a)}", b""),
        b"\n1\n"
    );
}

#[test]
fn convfmt_is_used_for_comparisons_fields_and_keys() {
    assert_eq!(
        common::run_with_stdin(
            "BEGIN{CONVFMT=\"%.2f\";print (1.234==\"1.23\");$0=\"x\";$1=1.234;print $0;a[1.234]=7;print a[\"1.23\"],sprintf(\"%s\",1.234)}",
            b""
        ),
        b"1\n1.23\n7 1.23\n"
    );
}
