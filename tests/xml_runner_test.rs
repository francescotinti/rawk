use rawk::test_support as harness;
use std::path::Path;
use std::time::Duration;

#[test]
fn run_xml_testsuite() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/testsuite.xml");
    let cases = harness::load(&manifest).unwrap();
    let mut failures = Vec::new();
    for case in &cases {
        let outcome = harness::execute(
            Path::new(env!("CARGO_BIN_EXE_rawk")),
            case,
            Duration::from_secs(3),
        )
        .unwrap();
        if !harness::validate(case, &outcome) {
            failures.push(format!("{}: {:?}", case.name, outcome));
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures out of {}:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
