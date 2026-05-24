// Step 20 Phase 1 — diffrun bucket output contract.
// Step 21 — stale SKIP retirement: 2 testcase (0006 bitwise, 0012 srand-rng)
// promossi da SKIPPED a EXPECTED-DIVERGE.
//
// Pre-step20 (RED): diffrun emits a single `DIVERGE:` bucket.
// Post-step20 Phase 1 (GREEN): diffrun splits divergences into
// `EXPECTED-DIVERGE:` (testcases marked `<expected_divergence reason="…"/>`)
// and `UNEXPECTED-DIVERGE:` (real regressions).
// Post-step21 (GREEN): counts == 96 / 10 / 0 / 3.

use std::process::Command;

fn run_diffrun() -> (String, std::process::ExitStatus) {
    let bin = env!("CARGO_BIN_EXE_diffrun");
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let output = Command::new(bin)
        .arg("tests/testsuite.xml")
        .current_dir(manifest_dir)
        .output()
        .expect("failed to spawn diffrun");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (stdout, output.status)
}

#[test]
fn diffrun_emits_four_bucket_summary() {
    let (stdout, _) = run_diffrun();
    assert!(
        stdout.contains("MATCH:"),
        "missing MATCH bucket\n---\n{stdout}"
    );
    assert!(
        stdout.contains("EXPECTED-DIVERGE:"),
        "missing EXPECTED-DIVERGE bucket — Phase 1 not implemented\n---\n{stdout}"
    );
    assert!(
        stdout.contains("UNEXPECTED-DIVERGE:"),
        "missing UNEXPECTED-DIVERGE bucket — Phase 1 not implemented\n---\n{stdout}"
    );
    assert!(
        stdout.contains("SKIPPED:"),
        "missing SKIPPED bucket\n---\n{stdout}"
    );
}

#[test]
fn diffrun_exits_nonzero_when_unexpected_divergences_present() {
    // Pre-Phase 2: the 8 known divergences are NOT yet annotated, so they
    // remain UNEXPECTED-DIVERGE and the binary must signal failure.
    // Post-Phase 2: all known divergences become EXPECTED-DIVERGE; this test
    // is updated to assert exit==0. We deliberately encode the Phase 1
    // invariant first.
    let (stdout, status) = run_diffrun();
    let unexpected = parse_bucket(&stdout, "UNEXPECTED-DIVERGE");
    if unexpected > 0 {
        assert!(
            !status.success(),
            "UNEXPECTED-DIVERGE={unexpected} but exit code is success\n---\n{stdout}"
        );
    } else {
        assert!(
            status.success(),
            "UNEXPECTED-DIVERGE=0 but exit code is failure\n---\n{stdout}"
        );
    }
}

#[test]
fn diffrun_step21_target_counts() {
    // Step 21 target after stale-skip retirement.
    // Phase 1 RED: con `is_skip()` ancora largo, SKIPPED=5 e MATCH+EXPECTED=104.
    // Phase 1 GREEN: bitwise+srand escono dallo skip e i due testcase
    // ricevono <expected_divergence/>, portando MATCH+EXPECTED a 106 e
    // SKIPPED a 3.
    //
    // Nota: MATCH ed EXPECTED-DIVERGE oscillano singolarmente per via di
    // `0017_test_gawk_manual_word_frequency.xml` (match="contains" +
    // `non-deterministic-iteration-order`), quindi assertiamo solo la loro
    // somma — che è l'invariante stabile.
    let (stdout, status) = run_diffrun();
    let m = parse_bucket(&stdout, "MATCH");
    let e = parse_bucket(&stdout, "EXPECTED-DIVERGE");
    let u = parse_bucket(&stdout, "UNEXPECTED-DIVERGE");
    let s = parse_bucket(&stdout, "SKIPPED");
    assert_eq!(
        (m + e, u, s),
        (106, 0, 3),
        "Step 21 target violato (atteso MATCH+EXPECTED=106, UNEXPECTED=0, SKIPPED=3; \
         attuali m={m} e={e} u={u} s={s})\n---\n{stdout}"
    );
    assert!(
        status.success(),
        "exit code non zero malgrado UNEXPECTED-DIVERGE=0\n---\n{stdout}"
    );
}

fn parse_bucket(stdout: &str, label: &str) -> u32 {
    for line in stdout.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(label) {
            let value = rest.trim_start_matches(':').trim();
            let first_token = value.split_whitespace().next().unwrap_or("");
            return first_token.parse().unwrap_or(0);
        }
    }
    0
}
