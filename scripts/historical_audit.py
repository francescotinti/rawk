"""Run the supplied bugs-fixed corpus; report differences without hiding them."""
from pathlib import Path
import json
import os
import signal
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def run(binary, arguments, cwd):
    child = subprocess.Popen([str(binary), *arguments], stdin=subprocess.DEVNULL,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                             cwd=cwd, env=dict(os.environ, LC_ALL='C'), start_new_session=True)
    timeout = False
    try:
        stdout, stderr = child.communicate(timeout=3)
    except subprocess.TimeoutExpired:
        timeout = True
        os.killpg(child.pid, signal.SIGKILL)
        stdout, stderr = child.communicate()
    finally:
        try:
            os.killpg(child.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    return dict(stdout_hex=stdout.hex(), stderr_hex=stderr.hex(),
                stdout=stdout.decode(errors='backslashreplace'),
                stderr=stderr.decode(errors='backslashreplace'),
                code=child.returncode, timeout=timeout)


def main():
    results = []
    for source in sorted((ROOT/'c_awk/bugs-fixed').glob('*.awk')):
        if source.name.startswith('.'):
            continue
        args = ['-f', str(source)]
        fixture = source.with_suffix('.in')
        if fixture.exists():
            args.append(str(fixture))
        row = dict(case=source.name)
        for name, binary in [('c', ROOT/'c_awk/a.out'), ('rust', ROOT/'rawk/target/release/rawk')]:
            with tempfile.TemporaryDirectory(prefix='rawk-history-') as cwd:
                row[name] = run(binary, args, cwd)
        row['same_output_and_status'] = all(row['c'][key] == row['rust'][key]
                                             for key in ('stdout_hex','code','timeout'))
        results.append(row)
        print(source.name, 'MATCH' if row['same_output_and_status'] else 'DIFFER',
              'status', row['c']['code'],row['rust']['code'])
    (ROOT/'rawk/diary/historical-results.json').write_text(json.dumps(results,indent=2,ensure_ascii=False)+'\n')
    print('MATCH',sum(r['same_output_and_status'] for r in results),'/',len(results))


if __name__ == '__main__':
    main()
