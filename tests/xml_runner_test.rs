use std::fs::File;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TestSuite {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "testcase")]
    testcases: Vec<TestCase>,
}

#[derive(Debug, Deserialize)]
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
struct ExpectedStdout {
    #[serde(rename = "@match", default = "default_match")]
    match_type: String,
    #[serde(rename = "$value")]
    content: String,
}

fn default_match() -> String {
    "exact".to_string()
}

#[test]
fn run_xml_testsuite() {
    // Read the XML file
    let mut file = File::open("tests/testsuite.xml").expect("Failed to open testsuite.xml");
    let mut xml_content = String::new();
    file.read_to_string(&mut xml_content).expect("Failed to read testsuite.xml");

    let suite: TestSuite = quick_xml::de::from_str(&xml_content)
        .expect("Failed to parse testsuite.xml");

    println!("Running Test Suite: {}", suite.name);
    let mut failed = 0;
    let total = suite.testcases.len();

    for case in suite.testcases {
        println!("  Test Case: {}", case.name);

        let mut cmd = Command::new(env!("CARGO_BIN_EXE_rawk"));
        for arg in &case.args {
            cmd.arg(arg);
        }
        cmd.arg(&case.awk);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().expect("Failed to spawn rawk process");

        if let Some(stdin_data) = &case.stdin {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(stdin_data.as_bytes()).expect("Failed to write to stdin");
            }
        }

        let output = child.wait_with_output().expect("Failed to wait on rawk");

        let stdout_str = String::from_utf8_lossy(&output.stdout);
        let stderr_str = String::from_utf8_lossy(&output.stderr);
        
        let mut case_failed = false;

        // Check stdout
        let expected_out = case.expected_stdout.content.trim_start(); // allow leading whitespace in CDATA
        
        if case.expected_stdout.match_type == "contains" {
            for line in expected_out.lines() {
                if !line.trim().is_empty() && !stdout_str.contains(line) {
                    println!("    [FAIL] stdout did not contain expected line: {}", line);
                    case_failed = true;
                }
            }
        } else {
            // Exact match (trim trailing newlines for safety)
            if stdout_str.trim() != expected_out.trim() {
                println!("    [FAIL] stdout exact match failed.");
                println!("      Expected:\n{}", expected_out);
                println!("      Got:\n{}", stdout_str);
                case_failed = true;
            }
        }

        // Check stderr if expected
        if let Some(expected_err) = &case.expected_stderr {
            if stderr_str.trim() != expected_err.trim() {
                println!("    [FAIL] stderr exact match failed.");
                println!("      Expected:\n{}", expected_err);
                println!("      Got:\n{}", stderr_str);
                case_failed = true;
            }
        } else {
            // If expected_stderr is not provided, we expect it to be empty
            if !stderr_str.trim().is_empty() {
                println!("    [FAIL] unexpected stderr output:\n{}", stderr_str);
                case_failed = true;
            }
        }
        
        if !output.status.success() {
            println!("    [FAIL] Process exited with failure status: {}", output.status);
            case_failed = true;
        }

        if case_failed {
            failed += 1;
        } else {
            println!("    [OK]");
        }
    }

    if failed > 0 {
        panic!("{} out of {} tests failed in suite '{}'", failed, total, suite.name);
    } else {
        println!("All {} tests passed in suite '{}'!", total, suite.name);
    }
}
