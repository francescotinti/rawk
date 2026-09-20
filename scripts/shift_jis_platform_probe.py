#!/usr/bin/env python3
"""Compare every libc-valid double-byte SJIS pair with the native C oracle.

The set is discovered on the executing host, never copied from Darwin tables.
This complements tests/shift_jis.rs (invalid input, BWK units, RS and NUL).
"""
import argparse
import ctypes
import hashlib
import json
import locale
import os
from pathlib import Path
import platform
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parent.parent
locale.setlocale(locale.LC_CTYPE, "ja_JP.SJIS")
libc = ctypes.CDLL(None)
libc.mbtowc.argtypes = [ctypes.POINTER(ctypes.c_wchar), ctypes.c_char_p, ctypes.c_size_t]
libc.mbtowc.restype = ctypes.c_int
pairs = []
for lead in range(128, 256):
    for trail in range(1, 256):
        if trail == 10:
            continue
        data = bytes([lead, trail])
        wc = ctypes.c_wchar()
        libc.mbtowc(None, None, 0)
        if libc.mbtowc(ctypes.byref(wc), data, len(data)) == 2:
            pairs.append(data)

binaries = {"c": root.parent / "c_awk" / "a.out", "rust": root / "target/release/rawk"}
data = b"".join(b"a" + pair + b"Z\n" for pair in pairs)
program = "{print toupper($0);print tolower($0)}"
outcomes = {}
for key, binary in binaries.items():
    result = subprocess.run([str(binary), program], input=data, capture_output=True,
                            env={**os.environ, "LC_ALL": "ja_JP.SJIS"}, timeout=30)
    outcomes[key] = {"code": result.returncode, "stdout_hex": result.stdout.hex(),
                     "stderr_hex": result.stderr.hex()}
passed = bool(pairs) and outcomes["c"]["code"] == 0 and not outcomes["c"]["stderr_hex"] and outcomes["c"] == outcomes["rust"]
summary = {
    "platform": platform.platform(), "machine": platform.machine(),
    "libc": platform.libc_ver(), "locale": "ja_JP.SJIS",
    "codeset": locale.nl_langinfo(locale.CODESET),
    "oracle_revision": subprocess.check_output(
        ["git", "-C", str(binaries["c"].parent), "rev-parse", "HEAD"], text=True).strip(),
    "rawk_revision": subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip(),
    "valid_double_byte_pairs": len(pairs), "conversion_match": passed,
    "input_sha256": hashlib.sha256(data).hexdigest(),
    "binary_sha256": {k: hashlib.sha256(p.read_bytes()).hexdigest() for k, p in binaries.items()},
    "outcomes": {k: {"code": v["code"], "stderr_hex": v["stderr_hex"],
                     "stdout_sha256": hashlib.sha256(bytes.fromhex(v["stdout_hex"])).hexdigest()}
                 for k, v in outcomes.items()},
}
if not passed:
    summary["failure"] = outcomes
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(summary, indent=2) + "\n")
print(f"{len(pairs)} libc-valid double-byte pairs: upper/lower {'MATCH' if passed else 'FAIL'}")
raise SystemExit(0 if passed else 1)
