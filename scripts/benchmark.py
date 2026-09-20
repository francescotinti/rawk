"""Interleaved, byte-checked macOS benchmark with configurable binaries and repetitions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
WORKLOADS = [
    ('sum_fields', '{s += $2} END{print s}', b'1 2 3\n' * 100000),
    ('regex_fields', 'BEGIN{FS="[,:]+"} {s += $2} END{print s}', b'a,2:c\n' * 10000),
    ('array_aggregation', '{a[$1]+=$2} END{print a["alpha"],a["beta"]}', b'alpha 2\nbeta 3\n' * 50000),
]
UTF8_WORKLOADS = [
    ('utf8_boolean_short', '{n+=($0 ~ /é|é😀/)} END{print n}', ('pré😀fin\n' * 100000).encode()),
    ('utf8_boolean_long', '{n+=($0 ~ /é.*|é/)} END{print n}', (('é' + 'λ' * 2048 + '\n') * 1000).encode()),
    ('utf8_boolean_miss', '{n+=($0 ~ /𐀀/)} END{print n}', ('Āabcé\n' * 100000).encode()),
    ('utf8_match', '{match($0,/é|é😀/);n+=RLENGTH} END{print n}', ('pré😀fin\n' * 100000).encode()),
    ('utf8_gsub', '{n+=gsub(/./,"x")} END{print n}', ('é😀λz\n' * 100000).encode()),
]


# Less uniform controls for field storage reuse: changing widths/counts, text,
# decimal/exponent values, empty records and saved values across records.
MIXED_INPUT = b"".join(
    (f"key{i % 127}\t{i % 997 - 498}.25  02 {i % 31}e-2 "
     + "extra " * (i % 7) + "\n").encode()
    if i % 19 else b" \t \n" for i in range(100000)
)
MIXED_WORKLOADS = [
    ('mixed_fields', '{s += $2; n += NF; last=$1} END{print s,n,last}', MIXED_INPUT),
    ('mixed_aggregation', '{a[$1]+=$2} END{print a["key0"],a["key126"],a[""]}', MIXED_INPUT),
]


UTF8_MIXED_INPUT = b"".join(
    (("pré😀fin" if i % 3 else "Āabc") + "λ" * (i % 17) + "\n").encode()
    for i in range(10000)
)
UTF8_MIXED_WORKLOADS = [
    ('utf8_dynamic_boolean', '{r=NR%2?"é|é😀":"𐀀";n+=($0~r)} END{print n}', UTF8_MIXED_INPUT),
    ('utf8_match_mixed', '{n+=match($0,/é|é😀|λ+/);l+=RLENGTH} END{print n,l}', UTF8_MIXED_INPUT),
    ('utf8_gsub_expanding', '{s=$0;n+=gsub(/é|é😀|λ+/,"<&>&",s);l+=length(s)} END{print n,l}', UTF8_MIXED_INPUT),
    ('utf8_gsub_miss', '{n+=gsub(/𐀀/,"x")} END{print n}', UTF8_MIXED_INPUT),
    ('utf8_match_long', '{n+=match($0,/é|é😀/);l+=RLENGTH} END{print n,l}',
     (("λ" * 2048 + "é😀fin\n") * 1000).encode()),
    ('utf8_empty_regex', '{s=$0;n+=($0~//);n+=match($0,//);n+=gsub(//,"x",s)} END{print n}', b"\n" * 100000),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', choices=['byte', 'utf8', 'mixed', 'utf8-mixed'], default='byte')
    parser.add_argument('--before', type=Path)
    parser.add_argument('--rawk', type=Path, default=ROOT / 'rawk/target/release/rawk')
    parser.add_argument('--output', type=Path, default=ROOT / 'rawk/diary/benchmark-consolidation.json')
    parser.add_argument('--runs', type=int, default=7)
    parser.add_argument('--scale', type=int, default=5)
    args = parser.parse_args()
    if args.runs < 1 or args.scale < 1:
        parser.error('runs and scale must be positive')
    binaries = [('c', ROOT / 'c_awk/a.out')]
    if args.before:
        binaries.append(('before', args.before.resolve()))
    binaries.append(('rust', args.rawk.resolve()))
    locale = 'en_US.UTF-8' if args.profile.startswith('utf8') else 'C'
    report = {'locale': locale, 'profile': args.profile, 'platform': platform.platform(), 'runs': args.runs, 'scale': args.scale,
              'binaries': {label: {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
                           for label, path in binaries}, 'workloads': []}
    for name, program, unit in {'byte': WORKLOADS, 'utf8': UTF8_WORKLOADS, 'mixed': MIXED_WORKLOADS, 'utf8-mixed': UTF8_MIXED_WORKLOADS}[args.profile]:
        data = unit * args.scale
        row = {'workload': name, 'program': program, 'records': data.count(b'\n'),
               'input_sha256': hashlib.sha256(data).hexdigest(),
               'runs': {label: {'seconds': [], 'max_rss_bytes': []} for label, _ in binaries}}
        expected = None
        # Warm each binary once; rotate order to reduce thermal/order bias.
        for repeat in range(-1, args.runs):
            order = binaries[repeat % len(binaries):] + binaries[:repeat % len(binaries)]
            for label, binary in order:
                start = time.perf_counter()
                result = subprocess.run(['/usr/bin/time', '-l', str(binary), program], input=data,
                                        capture_output=True, env=dict(os.environ, LC_ALL=locale), timeout=60)
                elapsed = time.perf_counter() - start
                if result.returncode:
                    raise RuntimeError(result.stderr)
                if expected is None:
                    expected = result.stdout
                if result.stdout != expected:
                    raise AssertionError((name, label, result.stdout, expected))
                match = re.search(rb'(\d+)\s+maximum resident set size', result.stderr)
                if repeat >= 0:
                    row['runs'][label]['seconds'].append(elapsed)
                    row['runs'][label]['max_rss_bytes'].append(int(match.group(1)) if match else None)
        row['stdout_hex'] = expected.hex()
        for values in row['runs'].values():
            values['median_seconds'] = statistics.median(values['seconds'])
            values['min_seconds'] = min(values['seconds'])
            values['max_seconds'] = max(values['seconds'])
            values['stdev_seconds'] = statistics.stdev(values['seconds']) if args.runs > 1 else 0
            rss = [v for v in values['max_rss_bytes'] if v is not None]
            values['median_max_rss_bytes'] = statistics.median(rss) if rss else None
        report['workloads'].append(row)
        print(name, {label: round(values['median_seconds'], 4) for label, values in row['runs'].items()}, flush=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
