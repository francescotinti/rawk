"""Harness contracts: actual native RSS, exact output, failures, timeout and statistics."""
from pathlib import Path
import subprocess
import tempfile
import unittest

from benchmark_native import compare, corpus, measure


class NativeBenchmarkTests(unittest.TestCase):
    def test_comparison(self):
        self.assertEqual(compare([1]*9, [2]*9)['signal'], 'slower')
        self.assertEqual(compare([2]*9, [1]*9)['signal'], 'faster')
        self.assertEqual(compare([1]*9, [1]*9)['signal'], 'inconclusive')
        with self.assertRaises(ValueError):
            compare([1], [2])

    def test_native_process(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'input'
            path.write_bytes(b'abc\x00\xff\n')
            for mode in ('file', 'pipe'):
                stdout, seconds, rss = self.run_script(tmp, 'cat', path, mode)
                self.assertEqual(stdout, path.read_bytes())
                self.assertGreater(seconds, 0)
                self.assertGreater(rss, 0)
            for body in ('echo bad >&2', 'exit 7'):
                with self.assertRaises(RuntimeError):
                    self.run_script(tmp, body, path, 'file')
            with self.assertRaises(subprocess.TimeoutExpired):
                self.run_script(tmp, 'sleep 10', path, 'file', timeout=0.05)

    def run_script(self, tmp, body, path, mode, timeout=5):
        binary = Path(tmp) / 'fake'
        binary.write_text('#!/bin/sh\n' + body + '\n')
        binary.chmod(0o755)
        return measure(binary, [], '', path, path.read_bytes(), mode, 'C', timeout)

    def test_corpus(self):
        rows = list(corpus())
        self.assertEqual(len(rows), 26)
        self.assertEqual(len({r[0] for r in rows}), len(rows))
        self.assertTrue(any(r[4] == 'pipe' for r in rows))
        self.assertTrue(any(r[5] != 'C' for r in rows))


if __name__ == '__main__':
    unittest.main()
