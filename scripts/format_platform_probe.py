#!/usr/bin/env python3
"""Record native BWK formatting evidence; never rewrite test expectations."""
import argparse
import json
import os
import platform
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
oracle = root.parent / "c_awk" / "a.out"
cases = []

def record(program):
    result = subprocess.run([str(oracle), program], capture_output=True,
                            env={**os.environ, "LC_ALL": "C"}, timeout=5)
    cases.append(dict(program=program, code=result.returncode,
                      stdout=result.stdout.hex(), stderr=result.stderr.hex()))

for flags in ["", "0", "+#0", " #", "-0"]:
    for width in [-9, 0, 9]:
        for conv in "diuoxXfgeE aAsc".replace(" ", ""):
            value = '\"abcd\"' if conv == "s" else "1.25"
            record(f'BEGIN{{printf "[%{flags}*.*{conv}] %d\\n",{width},-3,{value},73}}')
for value in ["-0.5", "-1", "-3.75", "-2147483649", "-9007199254740991",
              "0", "1", "9007199254740991", "9223372036854775808",
              "18446744073709549568"]:
    record(f'BEGIN{{printf "%u %o %x\\n",{value},{value},{value}}}')
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(dict(platform=platform.platform(),
    oracle_revision=subprocess.check_output(
        ["git", "-C", str(oracle.parent), "rev-parse", "HEAD"], text=True).strip(),
    cases=cases), indent=2) + "\n")
print(f"Recorded {len(cases)} native oracle cases in {args.output}")
