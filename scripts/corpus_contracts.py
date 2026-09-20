"""Narrow, versioned contracts for the six intentional corpus differences."""
from collections import Counter
import hashlib
import json
from pathlib import Path


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sections(data):
    outside, blocks, current = [], [], None
    for line in data.splitlines(keepends=True):
        if line == b'<<<\n':
            if current is not None: return None
            current = []
            outside.append(('block', len(blocks)))
        elif line == b'>>>\n':
            if current is None: return None
            blocks.append(Counter(current)); current = None
        elif current is None: outside.append(('line', line))
        else: current.append(line)
    return (outside, blocks) if current is None and blocks else None


def accepts(row, source_dir, contracts):
    contract = contracts.get(row['case'])
    if not contract or row['differences'] != ['stdout_hex']: return False
    for name, expected in contract['inputs'].items():
        if digest((source_dir/name).read_bytes()) != expected: return False
    for label in ['c','rust']:
        if row[label]['code'] != 0 or row[label]['timeout'] or row[label]['stderr_hex']: return False
    if row['c']['files'] != row['rust']['files']: return False
    a, b = (bytes.fromhex(row[label]['stdout_hex']) for label in ['c','rust'])
    rule = contract['rule']
    if rule == 'array-lines': return Counter(a.splitlines(keepends=True)) == Counter(b.splitlines(keepends=True))
    if rule == 'array-sections': return sections(a) is not None and sections(a) == sections(b)
    if rule in ['fixed-rng-streams', 'fixed-binary-streams']:
        return digest(a) == contract['stdout_sha256']['c'] and digest(b) == contract['stdout_sha256']['rust']
    return False


def load():
    return json.loads((Path(__file__).resolve().parents[1]/'tests/corpus-contracts.json').read_text())
