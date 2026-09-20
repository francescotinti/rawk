//! Byte-exact, isolated subprocess execution. Stdin/stdout/stderr use files so
//! neither a full pipe nor inherited pipe handles can defeat the timeout.
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub code: Option<i32>,
    pub timed_out: bool,
}

pub fn run(command: Command, input: &[u8], timeout: Duration) -> Result<Outcome> {
    Ok(run_with_fixtures(command, input, timeout, &[])?.0)
}

/// Copy named fixtures into the isolated working directory and capture files
/// produced by the program as well as its streams and exit status.
pub fn run_with_fixtures(
    mut command: Command,
    input: &[u8],
    timeout: Duration,
    fixtures: &[(&str, &[u8])],
) -> Result<(Outcome, std::collections::BTreeMap<String, Vec<u8>>)> {
    let work = tempfile::tempdir()?;
    for (name, bytes) in fixtures {
        anyhow::ensure!(
            Path::new(name)
                .components()
                .all(|c| matches!(c, std::path::Component::Normal(_))),
            "invalid fixture path"
        );
        fs::write(work.path().join(name), bytes)?;
    }
    let io = tempfile::tempdir()?;
    let input_path = io.path().join("stdin");
    fs::write(&input_path, input)?;
    let stdout_path = io.path().join("stdout");
    let stderr_path = io.path().join("stderr");
    // Default to the established byte profile, but preserve an explicitly set
    // or removed LC_ALL so locale-precedence tests exercise the requested mode.
    if !command.get_envs().any(|(key, _)| key == "LC_ALL") {
        command.env("LC_ALL", "C");
    }
    command
        .current_dir(work.path())
        .stdin(File::open(input_path)?)
        .stdout(File::create(&stdout_path)?)
        .stderr(File::create(&stderr_path)?);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().context("spawn test process")?;
    let start = Instant::now();
    let (status, timed_out) = loop {
        if let Some(status) = child.try_wait()? {
            break (status, false);
        }
        if start.elapsed() >= timeout {
            #[cfg(unix)]
            kill_group(child.id());
            let _ = child.kill();
            break (child.wait()?, true);
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    // A script may leave background children even when the parent exits normally.
    #[cfg(unix)]
    kill_group(child.id());
    let mut files = std::collections::BTreeMap::new();
    for entry in fs::read_dir(work.path())? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type()?.is_file() && !fixtures.iter().any(|(f, _)| *f == name) {
            files.insert(name, fs::read(entry.path())?);
        }
    }
    Ok((
        Outcome {
            stdout: fs::read(stdout_path)?,
            stderr: fs::read(stderr_path)?,
            code: status.code(),
            timed_out,
        },
        files,
    ))
}

#[cfg(unix)]
fn kill_group(pid: u32) {
    let _ = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "case")]
    cases: Vec<Entry>,
}
#[derive(Deserialize)]
struct Entry {
    #[serde(rename = "@file")]
    file: String,
    #[serde(rename = "@enabled", default)]
    enabled: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Case {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "args", default)]
    pub args: Vec<String>,
    pub awk: String,
    pub stdin: Option<String>,
    pub expected_stdout: ExpectedOutput,
    pub expected_stderr: Option<String>,
    #[serde(default)]
    pub expected_status: i32,
    pub expected_divergence: Option<Divergence>,
}
#[derive(Debug, Deserialize)]
pub struct ExpectedOutput {
    #[serde(rename = "@match", default)]
    pub mode: String,
    #[serde(rename = "$value", default)]
    pub content: String,
}
#[derive(Debug, Deserialize)]
pub struct Divergence {
    #[serde(rename = "@reason")]
    pub reason: String,
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

pub fn load(manifest: &Path) -> Result<Vec<Case>> {
    let data: Manifest = quick_xml::de::from_str(&fs::read_to_string(manifest)?)?;
    let parent = manifest.parent().unwrap_or(Path::new("."));
    data.cases
        .into_iter()
        .filter(|e| e.enabled.as_deref() != Some("false"))
        .map(|e| {
            let path = parent.join("cases").join(e.file);
            quick_xml::de::from_str(&fs::read_to_string(&path)?)
                .with_context(|| format!("case {}", path.display()))
        })
        .collect()
}

pub fn execute(binary: &Path, case: &Case, timeout: Duration) -> Result<Outcome> {
    let mut command = Command::new(binary);
    command.args(&case.args).arg(&case.awk);
    run(
        command,
        case.stdin.as_deref().unwrap_or("").as_bytes(),
        timeout,
    )
}

pub fn output_matches(expected: &[u8], actual: &[u8], mode: &str) -> bool {
    match mode {
        "" | "exact" => expected == actual,
        "unordered-lines" => {
            let lines = |b: &[u8]| {
                let mut lines: Vec<Vec<u8>> =
                    b.split_inclusive(|c| *c == b'\n').map(Vec::from).collect();
                lines.sort();
                lines
            };
            lines(expected) == lines(actual)
        }
        _ => false,
    }
}

pub fn validate(case: &Case, outcome: &Outcome) -> bool {
    !outcome.timed_out
        && outcome.code == Some(case.expected_status)
        && outcome.stderr == case.expected_stderr.as_deref().unwrap_or("").as_bytes()
        && output_matches(
            case.expected_stdout.content.as_bytes(),
            &outcome.stdout,
            &case.expected_stdout.mode,
        )
}

pub fn equivalent(case: &Case, left: &Outcome, right: &Outcome) -> bool {
    !left.timed_out
        && !right.timed_out
        && left.code == right.code
        && left.stderr == right.stderr
        && output_matches(&left.stdout, &right.stdout, &case.expected_stdout.mode)
}

pub fn expected_divergence(case: &Case, reference: &Outcome, binary: &Path) -> bool {
    let Some(expected) = &case.expected_divergence else {
        return false;
    };
    // Only the executable prefix is normalized; diagnostic text remains exact.
    let prefix = format!("{}:", binary.display());
    let stderr = reference
        .stderr
        .split_inclusive(|c| *c == b'\n')
        .flat_map(|line| {
            if let Some(rest) = line.strip_prefix(prefix.as_bytes()) {
                [b"<AWK>:".as_slice(), rest].concat()
            } else {
                line.to_vec()
            }
        })
        .collect::<Vec<_>>();
    !reference.timed_out
        && reference.code == Some(expected.status)
        && reference.stdout == expected.stdout.as_bytes()
        && stderr == expected.stderr.as_bytes()
}

pub fn reference_binary() -> Result<PathBuf> {
    let path = std::env::var_os("RAWK_REFERENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_awk/a.out"));
    if !path.is_file() {
        bail!(
            "reference AWK missing: {}; build c_awk or set RAWK_REFERENCE",
            path.display()
        );
    }
    Ok(path.canonicalize()?)
}
