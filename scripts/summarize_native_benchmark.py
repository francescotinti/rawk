"""Render complete native reports without comparing absolute timings across hosts."""
import argparse
import json
from pathlib import Path
import statistics


def render(paths):
    lines = ['# Native performance observations', '',
             'Each row compares paired binaries on one runner. Percentages are per-session',
             'median paired wall-time changes; RSS values are medians, not allocation counts.',
             'Signals are exploratory; inspect raw distributions and same-revision controls.', '']
    for path in paths:
        report = json.loads(path.read_text())
        if not report.get('complete') or not report.get('workloads'):
            raise ValueError(f'incomplete report: {path}')
        revisions = report['metadata']['revisions']
        lines += [f"## {report['platform']} ({report['machine']})", '',
                  f"Before `{revisions['before']}`; after `{revisions['after']}`.", '',
                  f"Source: `{path.name}`; compiler: `{report['metadata']['rustc'].splitlines()[0]}`.", '',
                  '| Case | Wall before → after seconds by session | Wall change by session | Wall signal | RSS before → after MiB by session |',
                  '|---|---|---|---|---|']
        for row in report['workloads']:
            changes = '; '.join(f"{100*(s['comparison']['seconds']['median_paired_ratio']-1):+.1f}%" for s in row['sessions'])
            seconds = '; '.join(f"{statistics.median(s['samples']['before']['seconds']):.4f} → {statistics.median(s['samples']['after']['seconds']):.4f}" for s in row['sessions'])
            memory = '; '.join(f"{statistics.median(s['samples']['before']['rss_bytes'])/2**20:.2f} → {statistics.median(s['samples']['after']['rss_bytes'])/2**20:.2f}" for s in row['sessions'])
            lines.append(f"| {row['name']} | {seconds} | {changes} | {row['seconds_assessment']} | {memory} |")
        lines.append('')
    return '\n'.join(lines)


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('reports', type=Path, nargs='+')
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    a.output.write_text(render(a.reports))


if __name__ == '__main__':
    main()
