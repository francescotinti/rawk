use proptest::prelude::*;
use rawk::test_support as harness;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

fn run_one(binary: &Path, program: &str) -> String {
    let mut cmd = Command::new(binary);
    cmd.arg(program);
    let out = harness::run(cmd, b"", Duration::from_secs(3)).unwrap();
    assert!(
        !out.timed_out && out.code == Some(0) && out.stderr.is_empty(),
        "{out:?}"
    );
    String::from_utf8(out.stdout).unwrap()
}

fn run_both(program: &str) -> (String, String) {
    (
        run_one(Path::new(env!("CARGO_BIN_EXE_rawk")), program),
        run_one(&harness::reference_binary().unwrap(), program),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn template_add(n in -1e6f64..1e6f64, m in -1e6f64..1e6f64) {
        let script = format!("BEGIN {{ print {} + {} }}", n, m);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_printf_int(n in -1_000_000_000i64..1_000_000_000i64) {
        let script = format!("BEGIN {{ printf \"%d\\n\", {} }}", n);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_printf_float(n in -1000.0f64..1000.0f64) {
        let script = format!("BEGIN {{ printf \"%.2f\\n\", {} }}", n);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_concat_string(s in "[a-zA-Z0-9 ]{0,8}", t in "[a-zA-Z0-9 ]{0,8}") {
        let script = format!("BEGIN {{ print \"{}\" \"{}\" }}", s, t);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_multiply(n in -1000i32..1000i32, m in -1000i32..1000i32) {
        let script = format!("BEGIN {{ print {} * {} }}", n, m);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_scientific_no_orphan_dot(n in 1e16f64..1e20f64) {
        // Note: NON differential (BSD awk e rawk divergono su large-int float printing).
        // Verifica solo la well-formedness dell'output di rawk per scientific.
        let script = format!("BEGIN {{ print {} }}", n);
        let out = run_one(Path::new(env!("CARGO_BIN_EXE_rawk")), &script);
        // Output must NOT contain ".e" or ".E" (orphan dot before exponent)
        prop_assert!(!out.contains(".e") && !out.contains(".E"),
                     "Orphan dot in scientific output: {} (script: {})", out, script);
    }
}
