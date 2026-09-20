"""Reproduce valid-pair coverage and the open C streaming-RS discrepancy.

Run from rawk: python3 diary/verification-shift-jis-2026-09-20/probe.py
The fresh probe process alone sets LC_CTYPE; rawk never sets a global locale.
"""
import ctypes
import hashlib
import json
import locale
import os
from pathlib import Path
import subprocess

locale.setlocale(locale.LC_CTYPE, "ja_JP.SJIS")
libc = ctypes.CDLL(None)
libc.mbtowc.argtypes = [ctypes.POINTER(ctypes.c_wchar), ctypes.c_char_p, ctypes.c_size_t]
libc.mbtowc.restype = ctypes.c_int
pairs = []
for lead in [*range(0x81, 0xA0), *range(0xE0, 0xFD)]:
    for trail in range(1, 256):
        if trail == 10:
            continue
        data = bytes([lead, trail])
        wc = ctypes.c_wchar()
        libc.mbtowc(None, None, 0)
        if libc.mbtowc(ctypes.byref(wc), data, 2) == 2:
            pairs.append(data)


def run(binary, program, data):
    result = subprocess.run(
        [binary, program], input=data, capture_output=True,
        env={**os.environ, "LC_ALL": "ja_JP.SJIS"}, timeout=30,
    )
    return {"code": result.returncode, "stdout_hex": result.stdout.hex(),
            "stderr_hex": result.stderr.hex()}


binaries = {"c": "../c_awk/a.out", "rust": "target/release/rawk"}
data = b"".join(b"a" + pair + b"Z\n" for pair in pairs)
program = "{print toupper($0);print tolower($0)}"
outcomes = {key: run(binary, program, data) for key, binary in binaries.items()}
assert pairs and outcomes["c"]["code"] == 0, outcomes["c"]
assert outcomes["rust"] == outcomes["c"], "valid pair conversion mismatch"
summary = {
    "locale": "ja_JP.SJIS", "valid_double_byte_pairs": len(pairs),
    "input_sha256": hashlib.sha256(data).hexdigest(),
    "conversion_match": True,
    "output_sha256": hashlib.sha256(bytes.fromhex(outcomes["c"]["stdout_hex"])).hexdigest(),
    "binary_sha256": {key: hashlib.sha256(Path(path).read_bytes()).hexdigest()
                      for key, path in binaries.items()},
    "rs_cases": [],
}
for seq in [b"\xc3\xa9", b"\xe1\x80\x80", b"\xf0\x90\x80\x80"]:
    for prefix in [b"", b"a", b"aa"]:
        data = prefix + seq + b"z\n"
        program = 'BEGIN{RS=".."}{print length($0),$0}'
        results = {key: run(binary, program, data) for key, binary in binaries.items()}
        summary["rs_cases"].append({"input_hex": data.hex(), "program": program,
                                    "match": results["c"] == results["rust"], **results})
Path(__file__).with_name("probe-results.json").write_text(json.dumps(summary, indent=2) + "\n")
print(f"{len(pairs)} valid double-byte pairs: upper/lower match")
print(f"RS mismatches: {sum(not case['match'] for case in summary['rs_cases'])}/9")
