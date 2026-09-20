import copy
import os
import unittest
from pathlib import Path
from corpus_contracts import accepts, load
from corpus_audit import audit, ROOT

class Contracts(unittest.TestCase):
    def test_each_contract_rejects_mutations(self):
        contracts = load()
        rows = audit(Path(os.environ.get('RAWK_BINARY', str(ROOT/'rawk/target/release/rawk'))), set(contracts))
        self.assertEqual({row['case'] for row in rows}, set(contracts))
        for row in rows:
            with self.subTest(case=row['case']):
                self.assertTrue(accepts(row, ROOT/'c_awk/testdir', contracts))
                for key,value in [('files', {'unexpected': '78'}),('code',3),('timeout',True),('stderr_hex','78'),('stdout_hex',row['rust']['stdout_hex']+'780a')]:
                    bad=copy.deepcopy(row);bad['rust'][key]=value
                    self.assertFalse(accepts(bad,ROOT/'c_awk/testdir',contracts))
                bad=copy.deepcopy(contracts);bad[row['case']]['inputs'][row['case']]='changed'
                self.assertFalse(accepts(row,ROOT/'c_awk/testdir',bad))

if __name__=='__main__':unittest.main()
