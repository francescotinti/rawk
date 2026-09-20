"""Local Darwin I/O benchmark: file and pipe input, rotated runs, C oracle, RSS."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import tempfile
import time


def workloads(suite):
    count = 'END{print NR,length($0)}'
    if suite == 'volume':
        yield 'volume_short_128m', [], count, b'abcdefghijklmno\n' * 2**23, 'file'
        yield 'volume_64k_128m', [], count, (b'x' * 65535 + b'\n') * 2048, 'file'
        return
    yield 'short_file', [], count, b'abcdefghijklmno\n' * 2**20, 'file'
    for size in (2**20, 4 * 2**20, 8 * 2**20):
        yield f'long_{size}_file', [], count, (b'x' * size + b'\n') * (16 * 2**20 // size), 'file'
    data = (b'x' * (4 * 2**20) + b'\n') * 4
    yield 'long_pipe', [], count, data, 'pipe'
    yield 'long_getline', [], 'BEGIN{while((getline x)>0){n++;s+=length(x)};print n,s}', data, 'file'
    yield 'unterminated', [], count, b'x' * (8 * 2**20), 'file'
    yield 'csv_long', ['--csv'], count, (b'"' + b'x' * (2**20) + b'\ny"\r\n') * 16, 'file'
    yield 'paragraph_control', [], 'BEGIN{RS=""} END{print NR,length($0)}', (b'x' * 2048 + b'\n\ny\n\n') * 1024, 'file'
    yield 'regex_control', [], 'BEGIN{RS="a+b"} {s+=length($0)} END{print NR,s}', (b'x' * 2048 + b'aaab') * 1024, 'file'


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--suite', choices=['records', 'volume'], default='records')
    p.add_argument('--workload', action='append', help='run only the named workload (repeatable)')
    p.add_argument('--before', type=Path)
    p.add_argument('--rawk', type=Path, default=Path('target/release/rawk'))
    p.add_argument('--oracle', type=Path, default=Path('../c_awk/a.out'))
    p.add_argument('--runs', type=int, default=9)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    if a.runs < 1:
        p.error('runs must be positive')
    binaries = [('c', a.oracle.resolve())]
    if a.before:
        binaries.append(('before', a.before.resolve()))
    binaries.append(('rust', a.rawk.resolve()))
    sha = lambda x: hashlib.sha256(x).hexdigest()
    report = {'suite': a.suite, 'platform': platform.platform(), 'locale': 'C', 'runs': a.runs,
              'method': 'file stdin descriptor or pipe communicate; wall includes time wrapper; time -l RSS bytes; warmup excluded',
              'binaries': {k: {'path': str(v), 'sha256': sha(v.read_bytes())} for k, v in binaries}, 'workloads': []}
    with tempfile.TemporaryDirectory(prefix='rawk-io-bench-') as directory:
        path = Path(directory) / 'input'
        selected = set(a.workload or [])
        seen = set()
        for name, flags, program, data, mode in workloads(a.suite):
            if selected and name not in selected:
                continue
            seen.add(name)
            path.write_bytes(data)
            # Establish oracle first, independently of rotated timing order.
            oracle = subprocess.run([str(a.oracle.resolve()), *flags, program, str(path)], capture_output=True, env=dict(os.environ, LC_ALL='C'), timeout=120, check=True)
            assert not oracle.stderr, oracle.stderr
            row = {'workload': name, 'program': program, 'flags': flags, 'input_mode': mode,
                   'input_bytes': len(data), 'input_sha256': sha(data), 'stdout_hex': oracle.stdout.hex(),
                   'runs': {k: {'seconds': [], 'max_rss_bytes': []} for k, _ in binaries}}
            for repeat in range(-1, a.runs):
                order = binaries[repeat % len(binaries):] + binaries[:repeat % len(binaries)]
                for label, binary in order:
                    with path.open('rb') as source:
                        start = time.perf_counter()
                        result = subprocess.run(['/usr/bin/time', '-l', str(binary), *flags, program],
                                                **({'input': data} if mode == 'pipe' else {'stdin': source}),
                                                capture_output=True, env=dict(os.environ, LC_ALL='C'), timeout=120)
                        elapsed = time.perf_counter() - start
                    assert result.returncode == 0 and result.stdout == oracle.stdout, (name, label, result)
                    match = re.search(rb'(\d+)\s+maximum resident set size', result.stderr)
                    assert match, result.stderr
                    if repeat >= 0:
                        row['runs'][label]['seconds'].append(elapsed)
                        row['runs'][label]['max_rss_bytes'].append(int(match[1]))
            for values in row['runs'].values():
                values['median_seconds'] = statistics.median(values['seconds'])
                values['min_seconds'] = min(values['seconds'])
                values['max_seconds'] = max(values['seconds'])
                values['stdev_seconds'] = statistics.stdev(values['seconds']) if a.runs > 1 else 0
                values['median_max_rss_bytes'] = statistics.median(values['max_rss_bytes'])
            report['workloads'].append(row)
            a.output.write_text(json.dumps(report, indent=2) + '\n')
            print(name, {k: round(v['median_seconds'], 4) for k, v in row['runs'].items()}, flush=True)
        if selected - seen:
            p.error(f'unknown workloads: {sorted(selected - seen)}')


if __name__ == '__main__':
    main()
