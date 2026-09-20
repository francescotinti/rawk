"""Record the clean oracle build used by the native performance workflow."""
import json
from pathlib import Path
import subprocess
import sys

work = Path(sys.argv[1])
path = work / 'metadata.json'
data = json.loads(path.read_text())
data['oracle'] = {
    'revision': subprocess.check_output(['git', '-C', str(work / 'oracle'), 'rev-parse', 'HEAD'], text=True).strip(),
    'status': subprocess.check_output(['git', '-C', str(work / 'oracle'), 'status', '--porcelain', '--untracked-files=no'], text=True),
    'build': 'make CC=cc CFLAGS=-O2',
    'version': subprocess.check_output([str(work / 'oracle/a.out'), '--version'], text=True).strip(),
}
path.write_text(json.dumps(data, indent=2) + '\n')
