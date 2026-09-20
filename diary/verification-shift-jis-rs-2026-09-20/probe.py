"""Seeded differential RS probe; run from rawk after cargo build --bins.

Accept an optional Rust binary path (default target/debug/rawk). Only writes
probe-results.json beside this script, preserving the earlier task's evidence.
"""
import hashlib
import json
import os
from pathlib import Path
import random
import subprocess
import sys

binaries = {"c": "../c_awk/a.out", "rust": sys.argv[1] if len(sys.argv) > 1 else "target/debug/rawk"}
patterns = [
    "..", "...", ".+", ".*", ".?", ".{1,3}", ".{2,4}", "[xz]+",
    "[^x]+", "[a-z]*", "[a-z].", "x.*z", "x.+z", "(.|x)z", "(x|..z)",
    "(x|x..z)", "x?", "x*", "^..", "..$", "^..$", "(x|$)", "(..)?",
    "(.x|..z)+", "a.*", "z+$", "[\\200-\\377]+", "\\u1000+", "\\u00e1.",
    "[^\\200-\\377]+", "(x|..)*z", "(a|aa)+",
]
chunks = [b"a", b"x", b"z", b"\n", b"\x82\xa0", b"\xc3\xa9",
          b"\xe1\x80\x80", b"\xf0\x90\x80\x80", b"\xe1\x80", b"\xe9", b"\xff"]
rng = random.Random(20260920)
inputs = [b"".join(rng.choices(chunks, k=rng.randrange(0, 18))) for _ in range(80)]
inputs += [prefix + seq + b"z\n" for seq in chunks[5:8] for prefix in [b"", b"a", b"aa"]]
failures = []
digest = hashlib.sha256()
total = 0
for pattern in patterns:
    # Preserve regex escapes through AWK's string-literal scanner.
    literal = pattern.replace("\\", "\\\\").replace('"', '\\"')
    program = f'BEGIN{{RS="{literal}"}}{{printf "[%s]",$0}}'
    for data in inputs:
        outcomes = {}
        for key, binary in binaries.items():
            p = subprocess.run([binary, program], input=data, capture_output=True,
                               env={**os.environ, "LC_ALL": "ja_JP.SJIS"}, timeout=5)
            outcomes[key] = {"code": p.returncode, "stdout_hex": p.stdout.hex(),
                             "stderr_hex": p.stderr.hex()}
        total += 1
        digest.update(json.dumps([program, data.hex(), outcomes], sort_keys=True).encode())
        if outcomes["c"] != outcomes["rust"]:
            failures.append({"program": program, "input_hex": data.hex(), **outcomes})
summary = {"seed": 20260920, "cases": total, "matches": total - len(failures),
           "failures": failures, "results_sha256": digest.hexdigest(),
           "binary_sha256": {key: hashlib.sha256(Path(path).read_bytes()).hexdigest()
                             for key, path in binaries.items()}}
Path(__file__).with_name("probe-results.json").write_text(json.dumps(summary, indent=2) + "\n")
print(f"{summary['matches']}/{total} MATCH, {len(failures)} failures", flush=True)
for failure in failures[:3]:
    print(failure, flush=True)
sys.exit(bool(failures))
