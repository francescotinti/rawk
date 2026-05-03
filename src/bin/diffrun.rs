use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio, exit};
use serde::Deserialize;

// duplicato da tests/xml_runner_test.rs (DRY trade-off accettato)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TestSuite {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "testcase")]
    testcases: Vec<TestCase>,
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
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ExpectedStdout {
    #[serde(rename = "@match", default = "default_match")]
    match_type: String,
    #[serde(rename = "$value")]
    content: String,
}

fn default_match() -> String {
    "exact".to_string()
}

fn run_cmd(mut cmd: Command, stdin_data: Option<&String>) -> Option<(String, String)> {
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Ok(mut child) = cmd.spawn() {
        if let Some(stdin_data) = stdin_data {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(stdin_data.as_bytes());
            }
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

fn is_skip(case: &TestCase) -> Option<&'static str> {
    let script = &case.awk;
    if script.contains("systime") || script.contains("strftime") {
        return Some("uses gawk-only systime/strftime");
    }
    if script.contains("and(") || script.contains("or(") || script.contains("xor(") || script.contains("lshift(") || script.contains("rshift(") {
        return Some("uses gawk-only bitwise functions");
    }
    if script.contains("gensub") || script.contains("mktime") {
        return Some("uses gawk-only gensub/mktime");
    }
    if script.contains("RT") {
        return Some("uses gawk-only RT variable");
    }
    if script.contains("BEGINFILE") || script.contains("ENDFILE") {
        return Some("uses gawk-only BEGINFILE/ENDFILE");
    }
    if script.contains("srand") {
        return Some("uses srand (might be non-deterministic or system-specific)");
    }
    None
}

fn main() {
    let awk_version_cmd = Command::new("awk").arg("--version").output();
    let awk_version = match awk_version_cmd {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout);
            s.lines().next().unwrap_or("unknown awk version").to_string()
        }
        Err(_) => {
            eprintln!("awk binary not found in PATH; install awk to use diffrun");
            exit(0);
        }
    };

    let mut file = File::open("tests/testsuite.xml").expect("Failed to open testsuite.xml");
    let mut xml_content = String::new();
    file.read_to_string(&mut xml_content).expect("Failed to read testsuite.xml");

    let suite: TestSuite = quick_xml::de::from_str(&xml_content)
        .expect("Failed to parse testsuite.xml");

    println!("=== rawk differential test report ===");
    println!("Reference awk: /usr/bin/awk ({})", awk_version);
    println!("Total testcase: {}", suite.testcases.len());

    let mut match_count = 0;
    let mut diverge_cases = Vec::new();
    let mut skipped_cases = Vec::new();

    for case in suite.testcases {
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
                let r_norm = if case.expected_stdout.match_type == "contains" { r_stdout.clone() } else { r_stdout.trim().to_string() };
                let a_norm = if case.expected_stdout.match_type == "contains" { a_stdout.clone() } else { a_stdout.trim().to_string() };
                
                if r_norm == a_norm {
                    match_count += 1;
                } else {
                    diverge_cases.push((case.name.clone(), r_stdout, a_stdout));
                }
            }
            _ => {
                diverge_cases.push((case.name.clone(), "Execution failed".to_string(), "Execution failed".to_string()));
            }
        }
    }

    println!("  MATCH:    {}", match_count);
    println!("  DIVERGE:  {}  (vedi sotto)", diverge_cases.len());
    println!("  SKIPPED:  {}  (vedi sotto)", skipped_cases.len());

    if !diverge_cases.is_empty() {
        println!("\n== DIVERGENCES ==");
        for (i, (name, r_stdout, a_stdout)) in diverge_cases.iter().enumerate() {
            println!("[{}] {}", i + 1, name);
            let r_lines: Vec<&str> = r_stdout.lines().take(3).collect();
            let a_lines: Vec<&str> = a_stdout.lines().take(3).collect();
            
            let r_preview = if r_lines.is_empty() { "\"\"" } else { r_stdout };
            let a_preview = if a_lines.is_empty() { "\"\"" } else { a_stdout };
            
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
}
