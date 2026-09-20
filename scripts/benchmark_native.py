"""Native paired benchmarks; build first, then run without concurrent build/test work."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import re
import statistics
import subprocess
import tempfile
import time

import benchmark
import benchmark_io


def sha(data):
    return hashlib.sha256(data).hexdigest()


def corpus():
    for group, rows in [('fields', benchmark.WORKLOADS + benchmark.MIXED_WORKLOADS),
                        ('utf8', benchmark.UTF8_MIXED_WORKLOADS)]:
        for name, program, data in rows:
            yield name, [], program, data, 'file', 'en_US.UTF-8' if group == 'utf8' else 'C'
    for suite in ('records', 'volume'):
        for row in benchmark_io.workloads(suite):
            yield (*row, 'C')
    yield ('csv_mixed', ['--csv'], '{n+=NF;s+=length($2)} END{print NR,n,s}',
           b'id,"hello, world",3\r\n2,"two\nlines",4\r\n3,"a""b",5\r\n' * 40000, 'file', 'C')
    yield ('high_cardinality', [], '{a[$1]+=$2} END{for(k in a){n++;s+=a[k]} print n,s}',
           b''.join(f'key{i%50000} {i%997}\n'.encode() for i in range(200000)), 'file', 'C')
    yield ('output_format', [], '{printf "%s:%.2f\\n",$1,$2/3}',
           b'alpha 123\nbeta -456\n' * 20000, 'file', 'C')


def compare(before, after):
    """Paired bootstrap interval describes this session, not cross-host causality."""
    if len(before) != len(after) or len(before) < 3 or any(x <= 0 for x in before + after):
        raise ValueError('need at least three positive paired samples')
    ratios = [b / a for a, b in zip(before, after)]
    rng = random.Random(1729)
    boot = sorted(statistics.median(rng.choices(ratios, k=len(ratios))) for _ in range(4000))
    lo, hi = boot[100], boot[3899]
    return {'median_paired_ratio': statistics.median(ratios), 'bootstrap_95_interval': [lo, hi],
            'signal': 'slower' if lo > 1 else 'faster' if hi < 1 else 'inconclusive'}


def measure(binary, flags, program, path, data, mode, locale, timeout):
    with tempfile.TemporaryDirectory() as tmp:
        timing = Path(tmp) / 'time'
        options = ['-l'] if platform.system() == 'Darwin' else ['-f', '%M']
        with path.open('rb') as source:
            start = time.perf_counter()
            proc = subprocess.Popen(['/usr/bin/time', *options, '-o', str(timing), str(binary), *flags, program],
                                    stdin=subprocess.PIPE if mode == 'pipe' else source,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                    env=dict(os.environ, LC_ALL=locale), start_new_session=True)
            try:
                stdout, stderr = proc.communicate(data if mode == 'pipe' else None, timeout=timeout)
            except subprocess.TimeoutExpired:
                import signal
                os.killpg(proc.pid, signal.SIGKILL)
                proc.communicate()
                raise
            elapsed = time.perf_counter() - start
        if proc.returncode or stderr:
            raise RuntimeError(f'{binary}: status={proc.returncode}, stderr={stderr!r}')
        raw = timing.read_text()
        match = re.search(r'(\d+)\s+maximum resident set size', raw) if platform.system() == 'Darwin' else re.fullmatch(r'(\d+)\s*', raw)
        if not match:
            raise RuntimeError(f'invalid RSS: {raw!r}')
        return stdout, elapsed, int(match[1]) * (1 if platform.system() == 'Darwin' else 1024)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--before', type=Path, required=True)
    p.add_argument('--after', type=Path, required=True)
    p.add_argument('--oracle', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--metadata', type=Path, required=True)
    p.add_argument('--runs', type=int, default=9)
    p.add_argument('--sessions', type=int, default=2)
    p.add_argument('--workload', action='append')
    a = p.parse_args()
    if a.runs < 3 or a.sessions < 1:
        p.error('runs >= 3 and sessions >= 1 required')
    binaries = {k: getattr(a, k).resolve() for k in ('oracle', 'before', 'after')}
    report = {'schema': 1, 'complete': False, 'metadata': json.loads(a.metadata.read_text()),
              'platform': platform.platform(), 'machine': platform.machine(),
              'method': 'rotated interleaved blocks, one excluded warmup per binary/case/session; wall includes wrapper and output capture; warm filesystem cache; RSS per process',
              'binaries': {k: {'sha256': sha(v.read_bytes()), 'path': str(v)} for k, v in binaries.items()},
              'workloads': []}
    a.output.parent.mkdir(parents=True, exist_ok=True)
    def save():
        a.output.write_text(json.dumps(report, indent=2) + '\n')
    save()
    selected = set(a.workload or [])
    seen = set()
    try:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'input'
            for name, flags, program, data, mode, locale in corpus():
                if selected and name not in selected:
                    continue
                seen.add(name)
                path.write_bytes(data)
                expected, _, _ = measure(binaries['oracle'], flags, program, path, data, mode, locale, 120)
                row = {'name': name, 'flags': flags, 'program': program, 'mode': mode, 'locale': locale,
                       'input_bytes': len(data), 'input_sha256': sha(data), 'output_sha256': sha(expected),
                       'output_bytes': len(expected), 'sessions': []}
                report['workloads'].append(row)
                for session in range(a.sessions):
                    samples = {k: {'seconds': [], 'rss_bytes': []} for k in binaries}
                    entry = {'samples': samples, 'order': []}
                    row['sessions'].append(entry)
                    for repeat in range(-1, a.runs):
                        labels = list(binaries)
                        offset = (repeat + session) % 3
                        labels = labels[offset:] + labels[:offset]
                        if repeat >= 0:
                            entry['order'].append(labels)
                        for label in labels:
                            out, seconds, rss = measure(binaries[label], flags, program, path, data, mode, locale, 120)
                            if out != expected:
                                raise RuntimeError(f'oracle mismatch: {name}/{label}: got sha256={sha(out)} expected={sha(expected)}')
                            if repeat >= 0:
                                samples[label]['seconds'].append(seconds)
                                samples[label]['rss_bytes'].append(rss)
                        save()
                    entry['comparison'] = {metric: compare(samples['before'][metric], samples['after'][metric])
                                           for metric in ('seconds', 'rss_bytes')}
                    print(name, session, entry['comparison'], flush=True)
                for metric in ('seconds', 'rss_bytes'):
                    signals = [s['comparison'][metric]['signal'] for s in row['sessions']]
                    row[metric + '_assessment'] = ('repeatable_' + signals[0] if len(signals) >= 2 and len(set(signals)) == 1 and signals[0] != 'inconclusive' else 'requires_confirmation')
                save()
        if selected - seen:
            raise ValueError(f'unknown workloads: {selected - seen}')
        report['complete'] = True
    except Exception as error:
        report['error'] = str(error)
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
