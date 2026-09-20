"""Build a disposable instrumented rawk; never use its timing as a benchmark.

Counts allocator requests (not libc-internal allocations), reader copies excluding
allocator-internal relocations, bytes examined, Read::read calls (including EOF) and buffer capacity.
The snapshot and injected source hashes make this source-level probe auditable.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

INSTRUMENT = r'''
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
pub static ALLOCS: AtomicUsize = AtomicUsize::new(0);
pub static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static REALLOCS: AtomicUsize = AtomicUsize::new(0);
pub static REALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static LIVE: AtomicUsize = AtomicUsize::new(0);
pub static PEAK: AtomicUsize = AtomicUsize::new(0);
pub static READS: AtomicUsize = AtomicUsize::new(0);
pub static READ_BYTES: AtomicUsize = AtomicUsize::new(0);
pub static SCANNED: AtomicUsize = AtomicUsize::new(0);
pub static COPIED: AtomicUsize = AtomicUsize::new(0);
pub static COMPACTED: AtomicUsize = AtomicUsize::new(0);
pub static CAPACITY: AtomicUsize = AtomicUsize::new(0);
fn grow(n: usize) { let live = LIVE.fetch_add(n, Relaxed) + n; PEAK.fetch_max(live, Relaxed); }
struct Count;
#[global_allocator] static ALLOCATOR: Count = Count;
unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() { ALLOCS.fetch_add(1, Relaxed); ALLOC_BYTES.fetch_add(l.size(), Relaxed); grow(l.size()); }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() { ALLOCS.fetch_add(1, Relaxed); ALLOC_BYTES.fetch_add(l.size(), Relaxed); grow(l.size()); }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        LIVE.fetch_sub(l.size(), Relaxed); unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, n) };
        if !q.is_null() {
            REALLOCS.fetch_add(1, Relaxed); REALLOC_BYTES.fetch_add(n, Relaxed);
            if n >= l.size() { grow(n-l.size()); } else { LIVE.fetch_sub(l.size()-n, Relaxed); }
        }
        q
    }
}
pub fn position(s: &[u8], b: u8) -> Option<usize> {
    let p = s.iter().position(|v| *v == b);
    SCANNED.fetch_add(p.map_or(s.len(), |p| p+1), Relaxed); p
}
pub fn copy(s: &[u8]) -> Vec<u8> { COPIED.fetch_add(s.len(), Relaxed); s.to_vec() }
pub fn report() {
    for (name, value) in [("allocations", &ALLOCS), ("allocated_bytes", &ALLOC_BYTES),
        ("reallocations", &REALLOCS), ("reallocation_requested_bytes", &REALLOC_BYTES),
        ("peak_live_requested_bytes", &PEAK), ("physical_reads", &READS),
        ("read_bytes", &READ_BYTES), ("separator_bytes_examined", &SCANNED),
        ("reader_explicit_copy_bytes", &COPIED), ("compacted_bytes", &COMPACTED),
        ("max_reader_capacity", &CAPACITY)] {
        eprintln!("IOPROFILE {} {}", name, value.load(Relaxed));
    }
}
'''


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--source', type=Path, default=Path('.'))
    p.add_argument('--work', type=Path, required=True)
    p.add_argument('--revision', help='profile tracked source at this Git revision')
    a = p.parse_args()
    a.work.mkdir(parents=True, exist_ok=False)
    source = a.source.resolve()
    snapshot = a.work / 'snapshot'
    snapshot.mkdir()
    names = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', a.revision or 'HEAD', 'src', 'Cargo.toml', 'Cargo.lock'], cwd=source, text=True).splitlines()
    for name in names:
        destination = snapshot / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(subprocess.check_output(['git', 'show', f'{a.revision}:{name}'], cwd=source) if a.revision else (source / name).read_bytes())
    sha = lambda b: hashlib.sha256(b).hexdigest()
    original = {str(v.relative_to(snapshot)): sha(v.read_bytes()) for v in snapshot.rglob('*') if v.is_file()}
    main_file = snapshot / 'src/main.rs'
    text = main_file.read_text().replace('mod ast;', 'mod io_profile;\nmod ast;')
    text = text.replace('std::process::exit(code);', 'io_profile::report();\n    std::process::exit(code);')
    main_file.write_text(text)
    (snapshot / 'src/io_profile.rs').write_text(INSTRUMENT)
    path = snapshot / 'src/input.rs'
    text = path.read_text()
    text = text.replace('self.buffer.drain(..self.start - 1);', 'crate::io_profile::COMPACTED.fetch_add(self.buffer.len() - (self.start - 1), std::sync::atomic::Ordering::Relaxed);\n            self.buffer.drain(..self.start - 1);')
    text = text.replace('let count = self.reader.read(&mut bytes)?;', 'crate::io_profile::READS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);\n        let count = self.reader.read(&mut bytes)?;\n        crate::io_profile::READ_BYTES.fetch_add(count, std::sync::atomic::Ordering::Relaxed);')
    text = text.replace('self.buffer.extend_from_slice(&bytes[..count]);', 'crate::io_profile::COPIED.fetch_add(count, std::sync::atomic::Ordering::Relaxed);\n        self.buffer.extend_from_slice(&bytes[..count]);\n        crate::io_profile::CAPACITY.fetch_max(self.buffer.capacity(), std::sync::atomic::Ordering::Relaxed);')
    text, n = re.subn(r'self.remaining\(\)(\[scanned\.\.\])?\s*\.iter\(\)\s*\.position\(\|b\| \*b == rs\[0\]\)', lambda m: 'crate::io_profile::position(&self.remaining()' + (m[1] or '') + ', rs[0])', text)
    assert n == 1, 'single-byte search changed; update probe explicitly'
    text = re.sub(r'(self.remaining\(\)(?:\[[^\]]*\])?)\.to_vec\(\)', r'crate::io_profile::copy(&\1)', text)
    text = text.replace('if *b == b\'"\' {', 'crate::io_profile::SCANNED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);\n                if *b == b\'"\' {')
    path.write_text(text)
    env = dict(os.environ, CARGO_INCREMENTAL='0', CARGO_TARGET_DIR='/tmp/rawk-io-profile-target')
    with (a.work / 'build.log').open('wb') as log:
        subprocess.run(['cargo', 'build', '--locked', '--release', '--bin', 'rawk'], cwd=snapshot, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = a.work / 'rawk-instrumented'
    shutil.copy2('/tmp/rawk-io-profile-target/release/rawk', binary)
    report = {'input_mode': 'regular file as stdin', 'revision': a.revision, 'original_sources': original, 'instrumented_input_sha256': sha(path.read_bytes()),
              'instrumentation_sha256': sha(INSTRUMENT.encode()), 'binary_sha256': sha(binary.read_bytes()),
              'rustc': subprocess.check_output(['rustc', '-vV'], text=True), 'cases': []}
    for name, flags, data in [
        ('short', [], b'abcdefghijklmno\n' * 65536),
        ('long', [], (b'x' * (2**20) + b'\n') * 4),
        ('unterminated', [], b'x' * (2**20)),
        ('csv', ['--csv'], (b'"' + b'x' * (2**20) + b'\ny"\r\n') * 4),
    ]:
        program = 'END{print NR,length($0)}'
        input_file = a.work / 'input'
        input_file.write_bytes(data)
        with input_file.open('rb') as stream:
            result = subprocess.run([str(binary.resolve()), *flags, program], stdin=stream, capture_output=True, env=dict(os.environ, LC_ALL='C'), check=True)
        oracle = subprocess.run([str((source.parent / 'c_awk/a.out').resolve()), *flags, program], input=data, capture_output=True, env=dict(os.environ, LC_ALL='C'), check=True)
        assert result.stdout == oracle.stdout and not oracle.stderr
        (a.work / f'{name}.stderr').write_bytes(result.stderr)
        report['cases'].append({'name': name, 'input_bytes': len(data), 'input_sha256': sha(data), 'stdout_hex': result.stdout.hex(), 'counters': {k.decode(): int(v) for k, v in re.findall(rb'IOPROFILE (\w+) (\d+)', result.stderr)}})
    (a.work / 'input').unlink()
    (a.work / 'profile.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report['cases'], indent=2))


if __name__ == '__main__':
    main()
