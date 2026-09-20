"""Build both immutable revisions with one pinned compiler in fresh directories."""
import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
import tarfile


def command(args, **kwargs):
    return subprocess.check_output(args, text=True, stderr=subprocess.STDOUT, **kwargs).strip()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--before', required=True)
    p.add_argument('--after', default='HEAD')
    p.add_argument('--toolchain', default='1.96.0')
    p.add_argument('--work', type=Path, required=True)
    a = p.parse_args()
    work = a.work.resolve()
    work.mkdir(parents=True, exist_ok=False)
    root = Path(__file__).resolve().parents[1]
    env = dict(os.environ, RUSTUP_TOOLCHAIN=a.toolchain, CARGO_INCREMENTAL='0', RUSTFLAGS='', CARGO_ENCODED_RUSTFLAGS='')
    # Fresh CARGO_HOME prevents user config from silently changing build flags.
    env['CARGO_HOME'] = str(work / 'cargo-home')
    metadata = {'toolchain': a.toolchain, 'rustc': command(['rustc', '-Vv'], env=env),
                'cargo': command(['cargo', '-V'], env=env), 'cc': command(['cc', '--version']),
                'platform': platform.platform(), 'machine': platform.machine(),
                'build_env': {k: env[k] for k in ('RUSTUP_TOOLCHAIN', 'CARGO_INCREMENTAL', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS')},
                'runner': {k: os.environ.get(k) for k in ('RUNNER_OS', 'RUNNER_ARCH', 'ImageOS', 'ImageVersion', 'GITHUB_RUN_ID')},
                'revisions': {}, 'command': 'cargo build --locked --release --bin rawk'}
    metadata['hardware'] = command(['sysctl', '-n', 'machdep.cpu.brand_string', 'hw.memsize']) if platform.system() == 'Darwin' else Path('/proc/cpuinfo').read_text() + Path('/proc/meminfo').read_text()
    for label, ref in [('before', a.before), ('after', a.after)]:
        revision = command(['git', 'rev-parse', ref + '^{commit}'], cwd=root)
        metadata['revisions'][label] = revision
        archive = work / (label + '.tar')
        subprocess.run(['git', 'archive', '-o', str(archive), revision], cwd=root, check=True)
        source = work / label
        source.mkdir()
        with tarfile.open(archive) as tar:
            tar.extractall(source, filter='data')
        build_env = dict(env, CARGO_TARGET_DIR=str(source / 'target'))
        with (work / (label + '-build.log')).open('w') as log:
            subprocess.run(['cargo', 'build', '--locked', '--release', '--bin', 'rawk'], cwd=source, env=build_env, stdout=log, stderr=subprocess.STDOUT, check=True)
    (work / 'metadata.json').write_text(json.dumps(metadata, indent=2) + '\n')


if __name__ == '__main__':
    main()
