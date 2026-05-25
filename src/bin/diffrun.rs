use serde::Deserialize;
use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio, exit};

// duplicato da tests/xml_runner_test.rs (DRY trade-off accettato)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Manifest {
    #[serde(rename = "case")]
    cases: Vec<ManifestCase>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ManifestCase {
    #[serde(rename = "@file")]
    file: String,
    #[serde(rename = "@enabled", default = "default_enabled")]
    enabled: String,
}

fn default_enabled() -> String {
    "true".to_string()
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TestCase {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "args", default)]
    args: Vec<String>,
    awk: String,
    stdin: Option<String>,
    expected_stdout: ExpectedStdout,
    expected_stderr: Option<String>,
    expected_divergence: Option<ExpectedDivergence>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ExpectedStdout {
    #[serde(rename = "@match", default = "default_match")]
    match_type: String,
    #[serde(rename = "$value")]
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ExpectedDivergence {
    #[serde(rename = "@reason")]
    reason: String,
}

fn default_match() -> String {
    "exact".to_string()
}

fn run_cmd(mut cmd: Command, stdin_data: Option<&String>) -> Option<(String, String)> {
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Ok(mut child) = cmd.spawn() {
        if let Some(stdin_data) = stdin_data
            && let Some(mut stdin) = child.stdin.take()
        {
            let _ = stdin.write_all(stdin_data.as_bytes());
        }

        // Wait with timeout would be ideal, but for now we just wait_with_output.
        // POSIX awk might hang if stdin is missing but expected.
        if let Ok(output) = child.wait_with_output() {
            let stdout_str = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr_str = String::from_utf8_lossy(&output.stderr).into_owned();
            return Some((stdout_str, stderr_str));
        }
    }
    None
}

fn is_skip(_case: &TestCase) -> Option<&'static str> {
    // Step 23 chiude la serie 21→22→23: tutte le heuristic stale (bitwise,
    // srand, RT, BEGINFILE/ENDFILE) sono state ritirate. Le divergenze note
    // sono ora annotate caso-per-caso via `<expected_divergence reason="…"/>`
    // sui rispettivi XML; nessun script viene più scartato a priori.
    None
}

fn main() {
    let awk_version_cmd = Command::new("awk").arg("--version").output();
    let awk_version = match awk_version_cmd {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout);
            s.lines()
                .next()
                .unwrap_or("unknown awk version")
                .to_string()
        }
        Err(_) => {
            eprintln!("awk binary not found in PATH; install awk to use diffrun");
            exit(0);
        }
    };

    let mut file = File::open("tests/testsuite.xml").expect("Failed to open testsuite.xml");
    let mut xml_content = String::new();
    file.read_to_string(&mut xml_content)
        .expect("Failed to read testsuite.xml");

    let manifest: Manifest =
        quick_xml::de::from_str(&xml_content).expect("Failed to parse manifest");

    let active: Vec<&ManifestCase> = manifest
        .cases
        .iter()
        .filter(|c| c.enabled != "false")
        .collect();

    println!("=== rawk differential test report ===");
    println!("Reference awk: /usr/bin/awk ({})", awk_version);
    println!("Total testcase: {}", active.len());

    let mut match_count = 0;
    let mut expected_diverge_cases: Vec<(String, String, String, String)> = Vec::new();
    let mut unexpected_diverge_cases: Vec<(String, String, String)> = Vec::new();
    let mut skipped_cases = Vec::new();

    for mc in active {
        let case_path = format!("tests/cases/{}", mc.file);
        let case_xml = std::fs::read_to_string(&case_path)
            .unwrap_or_else(|_| panic!("Failed to read {}", case_path));
        let case: TestCase = quick_xml::de::from_str(&case_xml)
            .unwrap_or_else(|_| panic!("Failed to parse {}", case_path));

        if let Some(reason) = is_skip(&case) {
            skipped_cases.push((case.name.clone(), reason));
            continue;
        }

        // Run rawk
        let rawk_path = std::env::current_exe().unwrap().with_file_name("rawk");
        let mut rawk_cmd = Command::new(&rawk_path);
        for arg in &case.args {
            rawk_cmd.arg(arg);
        }
        rawk_cmd.arg(&case.awk);
        let rawk_out = run_cmd(rawk_cmd, case.stdin.as_ref());

        // Run system awk
        let mut awk_cmd = Command::new("awk");
        for arg in &case.args {
            awk_cmd.arg(arg);
        }
        awk_cmd.arg(&case.awk);
        let awk_out = run_cmd(awk_cmd, case.stdin.as_ref());

        match (rawk_out, awk_out) {
            (Some((r_stdout, _)), Some((a_stdout, _))) => {
                let r_norm = if case.expected_stdout.match_type == "contains" {
                    r_stdout.clone()
                } else {
                    r_stdout.trim().to_string()
                };
                let a_norm = if case.expected_stdout.match_type == "contains" {
                    a_stdout.clone()
                } else {
                    a_stdout.trim().to_string()
                };

                if r_norm == a_norm {
                    match_count += 1;
                } else if let Some(ed) = &case.expected_divergence {
                    expected_diverge_cases.push((
                        case.name.clone(),
                        r_stdout,
                        a_stdout,
                        ed.reason.clone(),
                    ));
                } else {
                    unexpected_diverge_cases.push((case.name.clone(), r_stdout, a_stdout));
                }
            }
            _ => {
                let payload = (
                    case.name.clone(),
                    "Execution failed".to_string(),
                    "Execution failed".to_string(),
                );
                if let Some(ed) = &case.expected_divergence {
                    expected_diverge_cases.push((
                        payload.0,
                        payload.1,
                        payload.2,
                        ed.reason.clone(),
                    ));
                } else {
                    unexpected_diverge_cases.push(payload);
                }
            }
        }
    }

    println!("  MATCH:               {}", match_count);
    println!("  EXPECTED-DIVERGE:    {}", expected_diverge_cases.len());
    println!("  UNEXPECTED-DIVERGE:  {}", unexpected_diverge_cases.len());
    println!("  SKIPPED:             {}", skipped_cases.len());

    if !unexpected_diverge_cases.is_empty() {
        println!("\n== UNEXPECTED DIVERGENCES (regressions) ==");
        for (i, (name, r_stdout, a_stdout)) in unexpected_diverge_cases.iter().enumerate() {
            println!("[{}] {}", i + 1, name);
            let r_preview = if r_stdout.is_empty() {
                "\"\""
            } else {
                r_stdout
            };
            let a_preview = if a_stdout.is_empty() {
                "\"\""
            } else {
                a_stdout
            };
            println!("    rawk:    {:?}", r_preview);
            println!("    awk:     {:?}", a_preview);
        }
    }

    if !expected_diverge_cases.is_empty() {
        println!("\n== EXPECTED DIVERGENCES (annotated) ==");
        for (i, (name, r_stdout, a_stdout, reason)) in expected_diverge_cases.iter().enumerate() {
            println!("[{}] {}  ({})", i + 1, name, reason);
            let r_preview = if r_stdout.is_empty() {
                "\"\""
            } else {
                r_stdout
            };
            let a_preview = if a_stdout.is_empty() {
                "\"\""
            } else {
                a_stdout
            };
            println!("    rawk:    {:?}", r_preview);
            println!("    awk:     {:?}", a_preview);
        }
    }

    if !skipped_cases.is_empty() {
        println!("\n== SKIPPED ==");
        for (i, (name, reason)) in skipped_cases.iter().enumerate() {
            println!("[{}] {} — {}", i + 1, name, reason);
        }
    }

    if !unexpected_diverge_cases.is_empty() {
        exit(1);
    }
}
