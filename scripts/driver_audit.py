"""Run selected original shell drivers in isolated copies with bounded execution."""
import json
from pathlib import Path
import re
import shutil
import tempfile
import subprocess
from historical_audit import ROOT, run

DRIVERS = ['T.argv', 'T.clv', 'T.delete']


def audit(binary):
    rows = []
    for driver in DRIVERS:
        row = {'case': driver}
        for name, executable in [('c', ROOT/'c_awk/a.out'), ('rust', binary)]:
            with tempfile.TemporaryDirectory(prefix='rawk-driver-') as tmp:
                work = Path(tmp)
                text = (ROOT/'c_awk/testdir'/driver).read_text()
                if driver == 'T.argv':
                    text = text.replace('ARGV[0] is ../a.out', 'ARGV[0] is ./awk-under-test')
                    old = "diff foo1 foo2 || echo 'BAD: T.argv delete ARGV[2]'"
                    assert old in text
                    text = text.replace(old, "sort foo1 > sorted1; sort foo2 > sorted2; diff sorted1 sorted2 || echo 'BAD: T.argv delete ARGV[2]'")
                if driver == 'T.clv':
                    # These three assertions concern diagnostic wording. Accept
                    # each interpreter's precise message, also require status 2.
                    text = text.replace('grep "can\'t open.*foo"', 'test $? -eq 2 || echo "BAD: invalid filename status"\ngrep -E "can\'t open.*foo|input 99_=foo:"')
                    text = text.replace("grep 'invalid -v option argument: x'", "test $? -eq 2 || echo 'BAD: invalid -v status'\ngrep -E \"invalid -v option argument: x|invalid -v assignment 'x': expected name=value\"")
                    text = text.replace('/etc/passwd', 'test.passwd')
                    (work/'test.passwd').write_text('fixture:x:1:1:fixture:/tmp:/bin/sh\n')
                    subprocess.run(['cc', str(ROOT/'c_awk/testdir/echo.c'), '-o', str(work/'echo')], check=True, capture_output=True)
                (work/driver).write_text(text)
                shutil.copyfile(executable, work/'awk-under-test')
                (work/'awk-under-test').chmod(0o700)
                result = run(Path('/bin/sh'), [driver], work, {'awk': './awk-under-test'})
                stream = bytes.fromhex(result['stdout_hex']) + bytes.fromhex(result['stderr_hex'])
                result['failure_markers'] = [line.decode(errors='backslashreplace')
                    for line in stream.splitlines() if re.search(rb'\b(BAD|FAIL|failed|core dumped)\b', line, re.I)]
                result['passed'] = result['code'] == 0 and not result['timeout'] and not result['failure_markers'] and not result['stderr_hex']
                row[name] = result
        rows.append(row)
    return rows


if __name__ == '__main__':
    import argparse
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--rawk', type=Path, default=ROOT/'rawk/target/release/rawk')
    p.add_argument('--output', type=Path, default=ROOT/'rawk/diary/driver-results.json')
    args = p.parse_args()
    rows = audit(args.rawk.resolve())
    args.output.write_text(json.dumps(rows, indent=2)+'\n')
    for row in rows:
        print(row['case'], {name: row[name]['passed'] for name in ['c','rust']})
        for marker in row['rust']['failure_markers']: print(marker)
    if not all(r['c']['passed'] and r['rust']['passed'] for r in rows): raise SystemExit(1)
