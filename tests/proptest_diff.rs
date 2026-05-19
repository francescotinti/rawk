use proptest::prelude::*;
use std::process::{Command, Stdio};

fn awk_available() -> bool {
    Command::new("awk").arg("--version").output().is_ok()
}

fn run_both(awk_script: &str) -> (String, String) {
    let rawk_cmd = Command::new(env!("CARGO_BIN_EXE_rawk"))
        .arg(awk_script)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("Failed to run rawk");
    let rawk_out = String::from_utf8_lossy(&rawk_cmd.stdout).trim().to_string();

    let awk_cmd = Command::new("awk")
        .arg(awk_script)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .expect("Failed to run system awk");
    let awk_out = String::from_utf8_lossy(&awk_cmd.stdout).trim().to_string();

    (rawk_out, awk_out)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn template_add(n in -1e6f64..1e6f64, m in -1e6f64..1e6f64) {
        if !awk_available() {
            return Ok(());
        }
        let script = format!("BEGIN {{ print {} + {} }}", n, m);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_printf_int(n in -1_000_000_000i64..1_000_000_000i64) {
        if !awk_available() {
            return Ok(());
        }
        let script = format!("BEGIN {{ printf \"%d\\n\", {} }}", n);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_printf_float(n in -1000.0f64..1000.0f64) {
        if !awk_available() {
            return Ok(());
        }
        let script = format!("BEGIN {{ printf \"%.2f\\n\", {} }}", n);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_concat_string(s in "[a-zA-Z0-9 ]{0,8}", t in "[a-zA-Z0-9 ]{0,8}") {
        if !awk_available() {
            return Ok(());
        }
        let script = format!("BEGIN {{ print \"{}\" \"{}\" }}", s, t);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_multiply(n in -1000i32..1000i32, m in -1000i32..1000i32) {
        if !awk_available() {
            return Ok(());
        }
        let script = format!("BEGIN {{ print {} * {} }}", n, m);
        let (rawk, awk) = run_both(&script);
        prop_assert_eq!(rawk, awk, "Failed for script: {}", script);
    }

    #[test]
    fn template_scientific_no_orphan_dot(n in 1e16f64..1e20f64) {
        // Note: NON differential (BSD awk e rawk divergono su large-int float printing).
        // Verifica solo la well-formedness dell'output di rawk per scientific.
        let script = format!("BEGIN {{ print {} }}", n);
        let rawk_cmd = Command::new(env!("CARGO_BIN_EXE_rawk"))
            .arg(&script)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .expect("Failed to run rawk");
        let out = String::from_utf8_lossy(&rawk_cmd.stdout).trim().to_string();
        // Output must NOT contain ".e" or ".E" (orphan dot before exponent)
        prop_assert!(!out.contains(".e") && !out.contains(".E"),
                     "Orphan dot in scientific output: {} (script: {})", out, script);
    }
}
