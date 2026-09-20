"""Diagnostic native evidence for original-driver differences; no implicit exceptions."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile

from closure_cases import audit, check, ROOT

PROGRAMS = {
    'stdout-mixed': 'BEGIN { print "normal"; print "redirect" > "/dev/stdout"; print "after" }',
    'stdout-printf': 'BEGIN { printf "normal"; printf "redirect" > "/dev/stdout"; print "after" }',
    'stdout-append': 'BEGIN { print "normal"; print "append" >> "/dev/stdout"; print "after" }',
    'stderr-mixed': 'BEGIN { print "first" > "/dev/stderr"; print "second" >> "/dev/stderr"; print "last" > "/dev/stderr" }',
    'stdout-close': 'BEGIN { print "before"; s=close("/dev/stdout"); print s > "status"; print "hidden"; print "reopened" > "/dev/stdout" }',
    'stderr-close': 'BEGIN { s=close("/dev/stderr"); print s; print "hidden" > "/dev/stderr" }',
    'stdout-flush': 'BEGIN { printf "before"; print fflush("/dev/stdout"); print "after" > "/dev/stdout" }',
    'stderr-flush': 'BEGIN { print fflush("/dev/stderr"); print "after" > "/dev/stderr" }',
}


def streams(binary):
    rows = []
    for name, program in PROGRAMS.items():
        for mode in ['file', 'pipe']:
            with tempfile.TemporaryDirectory(prefix='rawk-stream-probe-') as tmp:
                work = Path(tmp)
                with (work/'stdout').open('wb') as out, (work/'stderr').open('wb') as err:
                    result = subprocess.run([str(binary), program], cwd=work,
                        env=dict(os.environ, LC_ALL='C'), stdin=subprocess.DEVNULL,
                        stdout=out if mode == 'file' else subprocess.PIPE,
                        stderr=err if mode == 'file' else subprocess.PIPE, timeout=5)
                rows.append(dict(case=name, mode=mode, program=program, code=result.returncode,
                    stdout_hex=((work/'stdout').read_bytes() if mode == 'file' else result.stdout).hex(),
                    stderr_hex=((work/'stderr').read_bytes() if mode == 'file' else result.stderr).hex(),
                    files={p.name: p.read_bytes().hex() for p in work.iterdir() if p.name not in ['stdout','stderr']}))
    return rows


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    binary = ROOT/'rawk/target/release/rawk'
    rows = audit(binary)
    contracts = json.loads((ROOT/'rawk/tests/closure-contracts.json').read_text())
    data = dict(platform=platform.platform(), machine=platform.machine(), libc=platform.libc_ver(),
        revision=subprocess.check_output(['git','-C',str(ROOT/'rawk'),'rev-parse','HEAD'],text=True).strip(),
        oracle_revision=subprocess.check_output(['git','-C',str(ROOT/'c_awk'),'rev-parse','HEAD'],text=True).strip(),
        binary_sha256={k:hashlib.sha256(p.read_bytes()).hexdigest() for k,p in [('c',ROOT/'c_awk/a.out'),('rust',binary)]},
        closure=rows, darwin_contract_mismatches=check(rows, contracts),
        streams={k:streams(p) for k,p in [('c',ROOT/'c_awk/a.out'),('rust',binary)]})
    args.output.write_text(json.dumps(data,indent=2)+'\n')
    print('Darwin contract mismatches:', data['darwin_contract_mismatches'])
    for c,r in zip(data['streams']['c'],data['streams']['rust']):
        if c != r: print('stream difference:',c['case'],c['mode'])
