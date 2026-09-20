"""Harness contracts: actual native RSS, exact output, failures, timeout and statistics."""
from pathlib import Path
import subprocess
import json
import locale
from unittest.mock import patch
import tempfile
import unittest

from benchmark_native import compare, corpus, measure, main, validate_locale
from summarize_native_benchmark import render


class NativeBenchmarkTests(unittest.TestCase):
    def test_locale(self):
        previous = locale.setlocale(locale.LC_CTYPE)
        validate_locale('C')
        with self.assertRaises(locale.Error):
            validate_locale('rawk_missing_locale.invalid')
        self.assertEqual(locale.setlocale(locale.LC_CTYPE), previous)

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

    def test_exact_output_failure_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            binaries = []
            for name, body in [('oracle', 'cat'), ('before', 'cat'), ('after', 'echo wrong')]:
                binary = tmp / name
                binary.write_text('#!/bin/sh\n' + body + '\n')
                binary.chmod(0o755)
                binaries.extend(['--' + name, str(binary)])
            metadata = tmp / 'metadata.json'
            metadata.write_text(json.dumps({'revisions': {'before': 'fixture', 'after': 'fixture'}, 'rustc': 'fixture'}))
            output = tmp / 'result.json'
            argv = ['benchmark_native', *binaries, '--metadata', str(metadata), '--output', str(output), '--runs', '3']
            with patch('sys.argv', argv), patch('benchmark_native.corpus', return_value=[('tiny', [], '', b'abc\n', 'file', 'C')]):
                with self.assertRaisesRegex(RuntimeError, 'oracle mismatch'):
                    main()
            report = json.loads(output.read_text())
            self.assertFalse(report['complete'])
            self.assertIn('oracle mismatch', report['error'])
            with self.assertRaisesRegex(ValueError, 'incomplete report'):
                render([output])
            (tmp / 'after').write_text('#!/bin/sh\ncat\n')
            with patch('sys.argv', argv), patch('benchmark_native.corpus', return_value=[('tiny', [], '', b'abc\n', 'file', 'C')]):
                main()
            complete = json.loads(output.read_text())
            self.assertTrue(complete['complete'])
            sessions = complete['workloads'][0]['sessions']
            self.assertEqual(len(sessions), 2)
            for session in sessions:
                self.assertEqual(len(session['order']), 3)
                self.assertEqual(len({tuple(order) for order in session['order']}), 3)
                for samples in session['samples'].values():
                    self.assertEqual(len(samples['seconds']), 3)
                    self.assertEqual(len(samples['rss_bytes']), 3)
            self.assertIn('| tiny |', render([output]))

    def test_corpus(self):
        rows = list(corpus())
        self.assertEqual(len(rows), 26)
        self.assertEqual(len({r[0] for r in rows}), len(rows))
        self.assertTrue(any(r[4] == 'pipe' for r in rows))
        self.assertTrue(any(r[5] != 'C' for r in rows))


if __name__ == '__main__':
    unittest.main()
