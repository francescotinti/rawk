//! Original p.* and t.* programs with the input conventions of Compare.p/t.
//! The manifest contains only verified cases, not generic expected failures.
use rawk::test_support as harness;
use std::{path::Path, process::Command, time::Duration};

#[test]
fn original_small_programs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_awk/testdir");
    let reference = harness::reference_binary().unwrap();
    let data = std::fs::read(root.join("test.data")).unwrap();
    let countries = std::fs::read(root.join("test.countries")).unwrap();
    for name in include_str!("corpus-verified.list")
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
    {
        let source = std::fs::read(root.join(name)).unwrap();
        let fixtures = [
            (name, source.as_slice()),
            ("test.data", data.as_slice()),
            ("test.countries", countries.as_slice()),
        ];
        let run = |binary: &Path| {
            let mut command = Command::new(binary);
            command.args(["-f", name]);
            if name.starts_with("p.") {
                command.args(["test.countries", "test.countries"]);
            } else {
                command.arg("test.data");
            }
            harness::run_with_fixtures(command, b"", Duration::from_secs(3), &fixtures).unwrap()
        };
        let expected = run(&reference);
        assert!(!expected.0.timed_out, "oracle timeout: {name}");
        let actual = run(Path::new(env!("CARGO_BIN_EXE_rawk")));
        assert_eq!(actual, expected, "original case: {name}");
    }
}
