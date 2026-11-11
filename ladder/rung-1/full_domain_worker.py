#!/usr/bin/env python3
"""Entire §5.1 D through public ISA action interface; no input/target/fault sampling."""
import argparse
import itertools
import json
from pathlib import Path
import subprocess
from gate import inputs, domain, check_action
from progress import write_summary
p=argparse.ArgumentParser(description=__doc__)
for key in ('executor','object','native','evidence','budget'):p.add_argument('--'+key,required=True)
p.add_argument('--arity',required=True,type=int)
a=p.parse_args();native=Path(a.native).read_bytes();evidence=Path(a.evidence)
summary={'cases':0,'faults':0,'reversed':0,'domain':domain(a.arity)}
with evidence.open('w') as log:
    for index,x in enumerate(inputs(a.arity)):
        for t,e in itertools.product(range(256),range(16)):
            argv=[a.executor,'reverse',a.object,'rung1_full','--mut',f'{t},{e}',
                  '--in',','.join(map(str,x)),'--arg','token:'+a.budget]
            r=subprocess.run(argv,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=30)
            log.write(json.dumps({'argv':argv,'exit':r.returncode,'stdout':r.stdout.decode(errors='replace'),
                                  'stderr':r.stderr.decode(errors='replace')})+'\n');log.flush()
            assert r.returncode==0,r.stderr.decode(errors='replace')
            v=json.loads(r.stdout)
            check_action(v,x,t,e,native[index])
            summary['cases']+=1;summary['reversed']+=1
            write_summary(evidence.with_name(evidence.stem+'-summary.json'),summary)
assert summary['cases']==domain(a.arity)['cases']
print(json.dumps(summary))
