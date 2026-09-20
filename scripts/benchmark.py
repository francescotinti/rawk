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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--profile', choices=['byte', 'utf8'], default='byte')
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
    locale = 'en_US.UTF-8' if args.profile == 'utf8' else 'C'
    report = {'locale': locale, 'profile': args.profile, 'platform': platform.platform(), 'runs': args.runs, 'scale': args.scale,
              'binaries': {label: {'path': str(path), 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
                           for label, path in binaries}, 'workloads': []}
    for name, program, unit in (UTF8_WORKLOADS if args.profile == 'utf8' else WORKLOADS):
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
        report['workloads'].append(row)
        print(name, {label: round(values['median_seconds'], 4) for label, values in row['runs'].items()}, flush=True)
    args.output.write_text(json.dumps(report, indent=2) + '\n')


if __name__ == '__main__':
    main()
