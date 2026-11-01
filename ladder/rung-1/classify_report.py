#!/usr/bin/env python3
"""Attach newly filed beads to unchanged real results, preserving the raw report."""
import argparse
import copy
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import sys
from gate import HERE, STAGES

def classify(report,known):
    result=copy.deepcopy(report)
    def leaves(rows):
        for row in rows.values():
            if row['status']=='FAIL':
                matched=[b['id'] for b in known if re.search(b['pattern'],row['error'])]
                row['beads']=matched or [b for b in row['beads'] if b!='UNFILED'] or ['UNFILED']
        for row in rows.values():
            if row['status']=='BLOCKED' and row['error'].startswith('Not run; requires '):
                deps=row['error'].removeprefix('Not run; requires ').split(', ')
                row['beads']=sorted({b for dep in deps for b in rows[dep]['beads']})
    for cases in result['cases'].values():
        for rows in cases.values():leaves(rows)
    for key,rows in result['stages'].items():
        cases=result['cases'][key]
        for stage,row in rows.items():
            if all(stage in cr for cr in cases.values()):
                row['beads']=sorted({b for cr in cases.values() for b in cr[stage]['beads']})
        leaves(rows)
    # Classification cannot alter a single measured outcome.
    for key,rows in report['stages'].items():
        assert [r['status'] for r in rows.values()]==[r['status'] for r in result['stages'][key].values()]
    for key,cases in result['cases'].items():
        for case,rows in cases.items():
            for stage,row in rows.items():
                assert row['status']==report['cases'][key][case][stage]['status']
    return result

def table(report):
    lines=['| Language / stage | LLVM 18 | LLVM 23 |','|---|---|---|']
    for lang in ('C','Rust','Julia'):
        for stage in STAGES:
            cells=[]
            for m in (18,23):
                r=report['stages'][f'{lang}/{m}'][stage]
                cells.append(r['status']+(' · '+', '.join(r['beads']) if r['beads'] else ''))
            lines.append('| '+lang+' / '+stage+' | '+' | '.join(cells)+' |')
    return '\n'.join(lines)+'\n'

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('report',type=Path);p.add_argument('--blockers',type=Path,default=HERE/'blockers.json')
    a=p.parse_args();raw=a.report.read_bytes();report=json.loads(raw)
    result=classify(report,json.loads(a.blockers.read_text()))
    for rows in list(result['stages'].values())+[r for cases in result['cases'].values() for r in cases.values()]:
        assert all(row['status']=='PASS' or row['beads'] and 'UNFILED' not in row['beads'] for row in rows.values()), 'unfiled real failure remains'
    original=a.report.with_name('report.unclassified.json')
    assert not original.exists(), 'already classified; use original data explicitly'
    original.write_bytes(raw)
    result['manifest']['bead_classification']={'time_utc':datetime.now(timezone.utc).isoformat(),
        'argv':[sys.executable,str(Path(__file__).resolve()),*sys.argv[1:]],
        'raw_report_sha256':hashlib.sha256(raw).hexdigest(),
        'blockers_sha256':hashlib.sha256(a.blockers.read_bytes()).hexdigest(),
        'note':'bead associations only; measured PASS/FAIL/BLOCKED statuses and commands unchanged'}
    a.report.write_text(json.dumps(result,indent=2)+'\n')
    a.report.with_name('table.md').write_text(table(result))
    print('Classified measured failures; statuses unchanged:',a.report)

if __name__=='__main__':main()
