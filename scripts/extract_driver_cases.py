"""Record reproducible invocations from mixed shell drivers, using the C oracle.

The checked-in result is reviewed test input, never an automatically accepted
expectation. Shell assertions remain visible in the full, unmodified audit.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
from full_driver_audit import ROOT, prepare, adapt
from historical_audit import run

WRAPPER = r'''import json, os, subprocess, sys
from pathlib import Path
stdin = sys.stdin.buffer.read()
work = Path(__file__).parent
args = [os.fsencode(a).replace(os.fsencode(work / 'test.passwd'), b'test.passwd') for a in sys.argv[1:]]
fixtures = {p.name: p.read_bytes().hex() for p in Path.cwd().glob('foo*') if p.is_file()}
fixtures['test.passwd'] = (work / 'test.passwd').read_bytes().hex()
row = dict(args_hex=[a.hex() for a in args], stdin_hex=stdin.hex(), fixtures=fixtures)
with (work / 'invocations.jsonl').open('a') as out: out.write(json.dumps(row)+'\n')
r = subprocess.run([str(work / 'reference'), *sys.argv[1:]], input=stdin)
sys.exit(r.returncode)
'''

if __name__ == '__main__':
    rows = []
    for driver in ['T.flags', 'T.misc', 'T.builtin', 'T.errmsg']:
        with tempfile.TemporaryDirectory(prefix='rawk-extract-') as tmp:
            work = Path(tmp)
            tests, tooling, fixture = prepare(work, ROOT / 'c_awk/a.out')
            (work / 'a.out').write_text('#!' + sys.executable + '\n' + WRAPPER)
            (work / 'a.out').chmod(0o700)
            script = tests / driver
            script.write_bytes(adapt(script.read_bytes(), driver, work, fixture))
            result = run(Path('/bin/sh'), [driver], tests,
                         {'awk': '../a.out', 'PATH': str(tooling)+os.pathsep+os.environ['PATH']}, timeout_seconds=60)
            if result['timeout']: raise RuntimeError(driver+' timed out')
            entries = [json.loads(line) for line in (work/'invocations.jsonl').read_text().splitlines()]
            for i, entry in enumerate(entries, 1):
                if driver == 'T.errmsg' and i <= 54: continue
                entry['id'] = f'{driver}/invocation-{i}'
                rows.append(entry)
            print(driver, len(entries))
    (ROOT/'rawk/tests/driver-subcases.json').write_text(json.dumps(rows, indent=2)+'\n')
