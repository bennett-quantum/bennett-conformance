#!/usr/bin/env python3
"""Input-only diagnostics. Never labelled full-D acceptance."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path
import subprocess
import time
from gate import inputs, result, restored
from progress import write_summary
p=argparse.ArgumentParser(description=__doc__)
for key in ('executor','object','native','evidence','budget'):p.add_argument('--'+key,required=True)
p.add_argument('--arity',required=True,type=int)
a=p.parse_args();expected=Path(a.native).read_bytes();evidence=Path(a.evidence)
summary={'cases':0,'faults':0,'mismatches':0,'reversed':0,'domain':'inputs only; t=e=0; NOT full D'}
summary_path=evidence.with_name(evidence.stem+'-summary.json')
def one(item):
    index,x=item
    argv=[a.executor,'reverse',a.object,'ref','--input',','.join(map(str,x)),'--budget',a.budget]
    start=time.monotonic();r=subprocess.run(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=30)
    return index,x,r,{'argv':argv,'exit':r.returncode,'elapsed_seconds':time.monotonic()-start,
                     'stdout':r.stdout.decode(errors='replace'),'stderr':r.stderr.decode(errors='replace')}
with evidence.open('w') as log:
    pool=ThreadPoolExecutor(max_workers=4)
    try:
        outcomes=pool.map(one,enumerate(inputs(a.arity)))
        for index,x,r,record in outcomes:
            log.write(json.dumps(record)+'\n');log.flush()
            assert r.returncode==0,r.stderr.decode(errors='replace')
            v=json.loads(r.stdout)
            if int(v['code']):summary['faults']+=1
            if result(v)!=expected[index]:summary['mismatches']+=1
            assert result(v)==expected[index],f'native mismatch at {x}: {v}'
            restored(v);summary['cases']+=1;summary['reversed']+=1
            if summary['cases']%256==0:write_summary(summary_path,summary)
    finally:
        pool.shutdown(wait=True,cancel_futures=True)
write_summary(summary_path,summary)
assert summary['cases']==len(expected)==256**a.arity
print(json.dumps(summary))
