"""Extract individual original expression/diagnostic cases without their legacy greps."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
from historical_audit import ROOT, run


def cases():
    source = ROOT / 'c_awk/testdir'
    body = (source / 'T.expr').read_bytes().split(b'<<\\!!!!\n')[1].split(b'\n!!!!')[0]
    group = number = 0
    program = None
    for line in body.splitlines():
        if line.startswith(b'try '):
            group += 1
            number = 0
            program = line[4:]
        elif not line or line.startswith(b'#'):
            continue
        elif program is not None:
            number += 1
            fields = line.split(b'\t')
            yield {'id': f'T.expr/{group}.{number}', 'args': [b'-F', b'\t', program], 'stdin': b'\t'.join(fields[:-1]) + b'\n'}
    body = (source / 'T.errmsg').read_bytes().split(b'<<\\!!!!\n')[1].split(b'\n!!!!')[0]
    for i, block in enumerate(body.split(b'\n\n'), 1):
        title, program = block.split(b'\n', 1)
        yield {'id': f'T.errmsg/{i}', 'description': title.decode(), 'args': [b'\n' + program], 'stdin': b'fixture\n'}


def extracted_cases():
    for case in json.loads((ROOT / 'rawk/tests/driver-subcases.json').read_text()):
        yield dict(id=case['id'], args=[bytes.fromhex(a) for a in case['args_hex']],
                   stdin=bytes.fromhex(case['stdin_hex']), fixtures=case['fixtures'])


def audit(binary):
    inventory = json.loads((ROOT / 'rawk/tests/driver-inventory.json').read_text())
    for name in ['T.expr', 'T.errmsg', 'T.flags', 'T.misc', 'T.builtin']:
        actual = hashlib.sha256((ROOT / 'c_awk/testdir' / name).read_bytes()).hexdigest()
        if actual != inventory[name]['source_sha256']:
            raise ValueError(f'original source changed: {name}')
    rows = []
    for case in [*cases(), *extracted_cases()]:
        row = {k: v for k, v in case.items() if k not in ['args', 'stdin']}
        row['args_hex'] = [arg.hex() for arg in case['args']]
        row['stdin_hex'] = case['stdin'].hex()
        row['fingerprint'] = hashlib.sha256(json.dumps([row['args_hex'], row['stdin_hex'], case.get('fixtures', {})]).encode()).hexdigest()
        for label, executable in [('c', ROOT / 'c_awk/a.out'), ('rust', binary)]:
            with tempfile.TemporaryDirectory(prefix='rawk-subcase-') as tmp:
                for name, data in case.get('fixtures', {}).items():
                    if Path(name).name != name: raise ValueError('unsafe fixture name')
                    (Path(tmp)/name).write_bytes(bytes.fromhex(data))
                result = run(executable, case['args'], Path(tmp), stdin_data=case['stdin'])
                stderr = bytes.fromhex(result['stderr_hex']).replace(str(executable).encode() + b':', b'<AWK>:').replace(b'rawk:', b'<AWK>:').replace(str(executable).encode(), b'<AWK>')
                result['stderr_hex'] = stderr.hex()
                result['stderr'] = stderr.decode(errors='backslashreplace')
                result['files'] = {p.name: p.read_bytes().hex() for p in Path(tmp).iterdir() if p.is_file() and p.read_bytes().hex() != case.get('fixtures', {}).get(p.name)}
                row[label] = result
        row['differences'] = [key for key in ['stdout_hex', 'stderr_hex', 'code', 'timeout', 'files'] if row['c'][key] != row['rust'][key]]
        rows.append(row)
    return rows


KEYS = ['stdout_hex', 'stderr_hex', 'code', 'timeout', 'files']


def signature(result):
    return {key: result[key] for key in KEYS}


def check(rows, contracts):
    if {r['id']: r['fingerprint'] for r in rows} != contracts['cases']:
        return ['case inventory or input fingerprint changed']
    errors = []
    for row in rows:
        expected = contracts['differences'].get(row['id'])
        if expected:
            good = all(signature(row[label]) == expected[label] for label in ['c', 'rust'])
        else:
            good = signature(row['c']) == signature(row['rust'])
        if not good or row['c']['timeout'] or row['rust']['timeout']:
            errors.append(row['id'])
    return errors


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rawk', type=Path, default=ROOT / 'rawk/target/debug/rawk')
    parser.add_argument('--output', type=Path, default=ROOT / 'rawk/diary/closure-cases.json')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    rows = audit(args.rawk.resolve())
    args.output.write_text(json.dumps(rows, indent=2) + '\n')
    for row in rows:
        if row['differences']:
            print(row['id'], row.get('description', ''), row['differences'], 'status', row['c']['code'], row['rust']['code'])
    print('cases', len(rows), 'exact', sum(not row['differences'] for row in rows))

    if args.check:
        errors = check(rows, json.loads((ROOT / 'rawk/tests/closure-contracts.json').read_text()))
        if errors:
            print('UNEXPECTED:', errors)
            raise SystemExit(1)
        print('All individual contracts passed')
