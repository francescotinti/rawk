"""Run original p.* / t.* with Compare.p / Compare.t fixture conventions.

Each process gets its own temporary directory. Compare exact streams, exit status
and generated files. Inventory every divergence; no automatic expected failures.
"""
import argparse
import collections
import json
import hashlib
from pathlib import Path
import shutil
import tempfile
from historical_audit import ROOT, run


def audit(binary, names=None):
    source_dir = ROOT / 'c_awk/testdir'
    reference = ROOT / 'c_awk/a.out'
    sources = sorted(p for p in source_dir.iterdir()
                     if p.is_file() and p.name.startswith(('p.', 't.')))
    if names is not None:
        sources = [p for p in sources if p.name in names]
    rows = []
    for source in sources:
        row = {'case': source.name}
        for label, executable in [('c', reference), ('rust', binary)]:
            with tempfile.TemporaryDirectory(prefix='rawk-corpus-') as tmp:
                work = Path(tmp)
                for name in [source.name, 'test.data', 'test.countries']:
                    shutil.copyfile(source_dir / name, work / name)
                args = ['-f', source.name]
                args += ['test.countries'] * 2 if source.name.startswith('p.') else ['test.data']
                result = run(executable, args, work)
                # C diagnostics prefix the executable path; Rust uses "rawk:".
                diagnostic = bytes.fromhex(result['stderr_hex']).replace(
                    str(executable).encode() + b':', b'<AWK>:')
                if label == 'rust':
                    diagnostic = diagnostic.replace(b'rawk:', b'<AWK>:')
                result['normalized_stderr_hex'] = diagnostic.hex()
                result['files'] = {str(p.relative_to(work)): p.read_bytes().hex()
                                   for p in sorted(work.rglob('*')) if p.is_file()
                                   and p.name not in [source.name, 'test.data', 'test.countries']}
                row[label] = result
        row['differences'] = [key for key in ('stdout_hex', 'normalized_stderr_hex', 'code', 'timeout', 'files')
                              if row['c'][key] != row['rust'][key]]
        row['status'] = ('reference-error' if row['c']['timeout'] or row['c']['code'] is None or (row['c']['code'] != 0 and row['c']['stderr_hex'])
                         else 'divergence' if row['differences'] else 'match')
        rows.append(row)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rawk', type=Path, default=ROOT/'rawk/target/release/rawk')
    parser.add_argument('--output', type=Path, default=ROOT/'rawk/diary/corpus-results.json')
    parser.add_argument('--check', action='store_true', help='Fail on any divergence or reference error')
    parser.add_argument('--full-output', action='store_true', help='Store complete streams as well as comparison results')
    args = parser.parse_args()
    rows = audit(args.rawk.resolve())
    if not args.full_output:
        for row in rows:
            for label in ['c', 'rust']:
                result = row[label]
                for stream in ['stdout', 'stderr']:
                    data = bytes.fromhex(result.pop(stream + '_hex'))
                    result.pop(stream)
                    result[stream] = {'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest(),
                                      'preview_hex': data[:256].hex()}
                result.pop('normalized_stderr_hex')
                result['files'] = {name: {'bytes': len(bytes.fromhex(hexdata)),
                                  'sha256': hashlib.sha256(bytes.fromhex(hexdata)).hexdigest()}
                                  for name, hexdata in result['files'].items()}
    args.output.write_text(json.dumps(rows, indent=2) + '\n')
    for row in rows:
        if row['status'] != 'match':
            print(row['case'], row['status'], ','.join(row['differences']))
    print(dict(collections.Counter(row['status'] for row in rows)), 'total', len(rows))
    if args.check and any(r['status'] != 'match' for r in rows):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
