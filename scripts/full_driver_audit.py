"""Inventory every original T.* driver; failures remain failures, including the oracle's."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
from historical_audit import ROOT, run

SOURCE = ROOT / 'c_awk/testdir'


def prepare(work, binary):
    tests = work / 'testdir'
    tests.mkdir()
    tracked = subprocess.check_output(['git', '-C', str(ROOT / 'c_awk'), 'ls-files', '-z', 'testdir']).split(b'\0')
    for relative in tracked:
        if not relative:
            continue
        source = ROOT / 'c_awk' / os.fsdecode(relative)
        if source.parent == SOURCE and source.is_file() and not source.name.startswith('.'):
            shutil.copyfile(source, tests / source.name)
    for archive in tests.glob('*.tar'):
        with tarfile.open(archive) as tar:
            for member in tar.getmembers():
                path = Path(member.name)
                if path.is_absolute() or '..' in path.parts or member.issym() or member.islnk():
                    raise ValueError(f'unsafe archive member: {member.name}')
    shutil.copyfile(binary, work / 'a.out')
    (work / 'a.out').chmod(0o700)
    shutil.copyfile(ROOT / 'c_awk/a.out', work / 'reference')
    (work / 'reference').chmod(0o700)
    tooling = work / 'bin'
    tooling.mkdir()
    (tooling / 'awk').symlink_to(work / 'reference')
    fixture = work / 'test.passwd'
    fixture.write_text(''.join(f'user{i}:x:{i}:1:fixture {i}:/tmp:/bin/sh\n' for i in range(1, 21)))
    subprocess.run(['cc', str(tests / 'echo.c'), '-o', str(tests / 'echo')], check=True, capture_output=True)
    return tests, tooling, fixture


def adapt(data, driver, work, fixture):
    data = data.replace(b'/etc/passwd', str(fixture).encode())
    data = data.replace(b'/tmp/awktestfoo', str(work / 'awktestfoo').encode())
    data = data.replace(b'/tmp/nawktest.XXXXXX', str(work / 'nawktest.XXXXXX').encode())
    data = data.replace(b'who >foo1', b'cat "' + str(fixture).encode() + b'" >foo1')
    data = data.replace(b'who | sed 10q', b'cat "' + str(fixture).encode() + b'" | sed 10q')
    if driver == 'T.arnold':
        # Darwin reports SIGABRT without the Linux core-dump bit. Only this
        # exact platform expectation changes; execution/status stay untouched.
        data = data.replace(b'cd arnold-fixes', b'cd arnold-fixes\nif [ "$(uname -s)" = Darwin ]; then\n  sed \'s/death by signal with core dump status 518/death by signal with core dump status 262/\' system-status.ok > system-status.darwin\n  mv system-status.darwin system-status.ok\nfi')
    if driver == 'T.beebe':
        data = data.replace(b"make all | sed 's/^/\t/' | grep -v cmp", b'make -ks all >make.log 2>&1; status=$?; cat make.log; exit "$status"')
    if driver == 'T.expr':
        # The outer program generates tests; only its subprocesses are subjects.
        data = data.replace(b"$awk '\nBEGIN", b"../reference '\nBEGIN", 1)
    if driver == 'T.argv':
        old = b"diff foo1 foo2 || echo 'BAD: T.argv delete ARGV[2]'"
        assert old in data
        data = data.replace(old, b"sort foo1 > sorted1; sort foo2 > sorted2; diff sorted1 sorted2 || echo 'BAD: T.argv delete ARGV[2]'")
    if driver == 'T.clv':
        data = data.replace(b'grep "can\'t open.*foo"', b'test $? -eq 2 || echo "BAD: invalid filename status"\ngrep -E "can\'t open.*foo|input 99_=foo:"')
        data = data.replace(b"grep 'invalid -v option argument: x'", b"test $? -eq 2 || echo 'BAD: invalid -v status'\ngrep -E \"invalid -v option argument: x|invalid -v assignment 'x': expected name=value\"")
    return data


def audit(binary, drivers, output, locale="C"):
    manifest = json.loads((ROOT / 'rawk/tests/reference-fixtures.json').read_text())
    actual = subprocess.check_output(['git', '-C', str(ROOT / 'c_awk'), 'ls-files', '-z', 'testdir']).split(b'\0')
    if {os.fsdecode(p) for p in actual if p} != set(manifest):
        raise ValueError('reference fixture inventory changed')
    for name, digest in manifest.items():
        if hashlib.sha256((ROOT / 'c_awk' / name).read_bytes()).hexdigest() != digest:
            raise ValueError(f'reference fixture changed: {name}')
    rows = []
    for driver in drivers:
        row = {'case': driver, 'locale': locale, 'source_sha256': hashlib.sha256((SOURCE / driver).read_bytes()).hexdigest()}
        for label, executable in [('c', ROOT / 'c_awk/a.out'), ('rust', binary)]:
            with tempfile.TemporaryDirectory(prefix='rawk-full-driver-') as tmp:
                work = Path(tmp)
                tests, tooling, fixture = prepare(work, executable)
                script = tests / driver
                script.write_bytes(adapt(script.read_bytes(), driver, work, fixture))
                result = run(Path('/bin/sh'), [driver], tests,
                             {'awk': '../a.out', 'oldawk': '../reference', 'LC_ALL': locale,
                              'PATH': str(tooling) + os.pathsep + os.environ['PATH']}, timeout_seconds=30)
                stream = bytes.fromhex(result['stdout_hex']) + bytes.fromhex(result['stderr_hex'])
                result['failure_markers'] = [line.decode(errors='backslashreplace') for line in stream.splitlines()
                    if re.search(rb'\b(BAD|FAIL|failed|fails|failure|core dumped|panic)\b', line, re.I)]
                result['passed'] = result['code'] == 0 and not result['timeout'] and not result['failure_markers'] and not result['stderr_hex']
                row[label] = result
        row['status'] = ('reference-failure' if not row['c']['passed'] else
                         'pass' if row['rust']['passed'] else 'open')
        rows.append(row)
        output.write_text(json.dumps(rows, indent=2) + "\n")
        print(driver, row['status'], ' '.join(row['rust']['failure_markers'])[:350], flush=True)
    return rows


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rawk', type=Path, default=ROOT / 'rawk/target/release/rawk')
    parser.add_argument('--output', type=Path, default=ROOT / 'rawk/diary/full-driver-results.json')
    parser.add_argument('--drivers', nargs='+')
    parser.add_argument('--verified', action='store_true', help='Run only drivers admitted by the explicit manifest')
    parser.add_argument('--check', action='store_true', help='Fail if any selected driver is not a pass')
    parser.add_argument('--locale', default='C', help='Locale used by both interpreters; byte gate defaults to C')
    args = parser.parse_args()
    drivers = args.drivers or sorted(p.name for p in SOURCE.glob('T.*') if not p.name.startswith('.'))
    if args.verified:
        drivers = (ROOT / 'rawk/tests/drivers-verified.list').read_text().splitlines()
    inventory = json.loads((ROOT / 'rawk/tests/driver-inventory.json').read_text())
    available = {p.name for p in SOURCE.glob('T.*')}
    if available != set(inventory):
        parser.error('original driver inventory changed; review additions/removals explicitly')
    admitted = {name for name, entry in inventory.items() if entry['gate']}
    manifest = set((ROOT / 'rawk/tests/drivers-verified.list').read_text().splitlines())
    if admitted != manifest:
        parser.error('verified driver manifest disagrees with inventory')
    for driver in drivers:
        if driver not in inventory or hashlib.sha256((SOURCE / driver).read_bytes()).hexdigest() != inventory[driver]['source_sha256']:
            parser.error(f'original source changed: {driver}')
    rows = audit(args.rawk.resolve(), drivers, args.output, args.locale)
    args.output.write_text(json.dumps(rows, indent=2) + '\n')
    if args.check and any(row['status'] != 'pass' for row in rows):
        raise SystemExit(1)
