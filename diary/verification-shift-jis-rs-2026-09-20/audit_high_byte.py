"""Validate C's signed-char ungetc failures without accepting arbitrary drift.

Run after probe.py. The probe patterns treat FF and FE identically (dot,
200-377 ranges, or ASCII/specific non-FF atoms); the generated inputs never
contain FE. Replace FF by FE only for this auxiliary C run, then restore FF
in its output and require an exact match with the stored Rust outcome.
This is an oracle workaround, explicitly distinct from an exact original run.
"""
import hashlib
import json
import os
from pathlib import Path
import subprocess

directory = Path(__file__).parent
source = json.loads((directory / "probe-results.json").read_text())
assert hashlib.sha256(Path("../c_awk/a.out").read_bytes()).hexdigest() == source["binary_sha256"]["c"]
checked = []
for case in source["failures"]:
    data = bytes.fromhex(case["input_hex"])
    assert b"\xfe" not in data and b"\xff" in data
    assert case["c"]["code"] == 2
    assert b"unable to ungetc '\xff'" in bytes.fromhex(case["c"]["stderr_hex"])
    assert case["rust"]["code"] == 0 and not case["rust"]["stderr_hex"]
    result = subprocess.run(["../c_awk/a.out", case["program"]],
                            input=data.replace(b"\xff", b"\xfe"), capture_output=True,
                            env={**os.environ, "LC_ALL": "ja_JP.SJIS"}, timeout=5)
    assert result.returncode == 0 and not result.stderr
    assert result.stdout.replace(b"\xfe", b"\xff").hex() == case["rust"]["stdout_hex"], case
    checked.append({"program": case["program"], "input_hex": case["input_hex"],
                    "substituted_c_stdout_hex": result.stdout.hex()})
summary = {"exact_matches": source["matches"], "known_c_ungetc_errors": len(checked),
           "auxiliary_matches": len(checked), "unexpected": 0, "cases": checked}
(directory / "high-byte-audit.json").write_text(json.dumps(summary, indent=2) + "\n")
print(f"{len(checked)} C ungetc errors independently checked; 0 unexpected differences")
