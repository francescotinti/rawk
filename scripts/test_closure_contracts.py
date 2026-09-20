"""Ensure exact exceptions cannot conceal new stream/status/file differences."""
import copy
import json
import unittest
from closure_cases import ROOT, check

class Contracts(unittest.TestCase):
    def test_each_exception_is_exact(self):
        contracts = json.loads((ROOT/'rawk/tests/closure-contracts.json').read_text())
        native = json.loads((ROOT/'rawk/tests/closure-contracts-linux-glibc.json').read_text())
        # Exercise both profiles on every platform, including negative probes.
        exceptions = [*contracts['differences'].items(), *native['differences'].items()]
        for name, expected in exceptions:
            row = dict(id=name, fingerprint=contracts['cases'][name], c=expected['c'], rust=expected['rust'])
            local = dict(cases={name: row['fingerprint']}, differences={name: expected})
            self.assertEqual(check([row], local), [])
            for label in ['c', 'rust']:
                for key, value in [('stdout_hex','424144'), ('stderr_hex','424144'), ('code',123), ('timeout',True), ('files',{'unexpected':'424144'})]:
                    changed = copy.deepcopy(row)
                    changed[label][key] = value
                    self.assertTrue(check([changed], local), (name,label,key))
            changed = dict(row, fingerprint='changed')
            self.assertTrue(check([changed], local))
            self.assertTrue(check([], local))
            self.assertTrue(check([row,dict(row,id='new-case')], local))
            self.assertTrue(check([row,row], local))

    def test_exact_cases_cannot_be_skipped_or_timeout(self):
        result = dict(stdout_hex='',stderr_hex='',code=0,timeout=False,files={})
        row = dict(id='case',fingerprint='f',c=result,rust=dict(result))
        contracts = dict(cases={'case':'f'},differences={})
        self.assertEqual(check([row],contracts),[])
        row['rust']['code'] = 2
        self.assertTrue(check([row],contracts))
        row['rust'] = row['c'] = dict(result,timeout=True)
        self.assertTrue(check([row],contracts))

if __name__ == '__main__': unittest.main()
