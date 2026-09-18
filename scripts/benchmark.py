"""Small reproducible macOS benchmarks; reports time and RSS, not a speed claim."""
import json
from pathlib import Path
import re
import statistics
import subprocess
import time
import os

ROOT=Path(__file__).resolve().parents[2]
WORKLOADS=[
    ('sum_fields','{s += $2} END{print s}',b'1 2 3\n'*100000),
    ('regex_fields','BEGIN{FS="[,:]+"} {s += $2} END{print s}',b'a,2:c\n'*10000),
    ('array_aggregation','{a[$1]+=$2} END{print a["alpha"],a["beta"]}',b'alpha 2\nbeta 3\n'*50000),
]
rows=[]
for name,program,data in WORKLOADS:
    row=dict(workload=name,records=data.count(b'\n'),runs={})
    for label,binary in [('c',ROOT/'c_awk/a.out'),('rust',ROOT/'rawk/target/release/rawk')]:
        seconds=[];memory=[]
        for _ in range(4):
            start=time.perf_counter()
            result=subprocess.run(['/usr/bin/time','-l',str(binary),program],input=data,
                                  capture_output=True,env=dict(os.environ,LC_ALL='C'),timeout=30)
            seconds.append(time.perf_counter()-start)
            if result.returncode:raise RuntimeError(result.stderr)
            match=re.search(rb'(\d+)\s+maximum resident set size',result.stderr)
            memory.append(int(match.group(1)) if match else None)
        row['runs'][label]=dict(seconds=seconds,median_seconds=statistics.median(seconds),
                               max_rss_bytes=memory,stdout=result.stdout.decode())
    assert row['runs']['c']['stdout']==row['runs']['rust']['stdout'],row
    rows.append(row)
(ROOT/'rawk/diary/benchmark-results.json').write_text(json.dumps(rows,indent=2)+'\n')
print(json.dumps(rows,indent=2))
