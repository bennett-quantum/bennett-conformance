"""Range-command scheduling tests with synthetic receipts; no acceptance claim."""
import argparse
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import gate


class RangeScheduling(unittest.TestCase):
    def setUp(self):
        self.temp=tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name)
        self.g=gate.Gate(argparse.Namespace(output=str(self.root/'runs'),timeout=300,full_d_jobs=2))
        self.exe=self.root/'executor';self.exe.write_bytes(b'synthetic pinned tool')
        self.g.executor=lambda name:self.exe
        self.calls=[]

    def prepare(self,case):
        d=self.g.folder('C',18)/case/'out';d.mkdir(parents=True)
        (d/'full-domain.bennett').write_bytes(b'synthetic admitted object')
        (self.g.folder('C',18)/(case+'-native.bin')).write_bytes(bytes(256**gate.CASES[case]))
        self.g.batch_available=lambda name:True

    def record(self,tag,argv,**kwargs):
        argv=list(map(str,argv));self.calls.append((tag,argv,kwargs))
        lo,hi=map(int,argv[argv.index('--range')+1].split('..'))
        path=Path(argv[argv.index('--evidence')+1])
        bindings={'executor_sha256':argv[argv.index('--executor-sha256')+1],
            'object_sha256':argv[argv.index('--object-sha256')+1],
            'native_sha256':gate.sha(Path(argv[argv.index('--native')+1])),
            'budget':argv[argv.index('--budget')+1]}
        path.write_text(json.dumps({'lo':lo,'hi':hi,'cases':hi-lo,'complete':True,**bindings}))
        return b'{}'

    def test_binary_and_unary_shards_cover_every_tuple_with_the_same_cap(self):
        self.g.command=self.record
        for case,shard_count in (('r1_sat_add',256),('r1_lookup',16)):
            self.prepare(case);self.calls.clear()
            self.g.exhaustive_D('C',18,case,'reference')
            self.g.domain_summary('C',18,case,'reference')
            self.assertEqual(len(self.calls),shard_count)
            for tag,argv,kwargs in self.calls:
                self.assertEqual(argv[argv.index('--timeout')+1],'300')
                self.assertNotIn('timeout',kwargs)
                self.assertIn('--identity',argv)
                self.assertIn('--executor-sha256',argv)
                self.assertIn('--object-sha256',argv)
            summary=json.loads((self.g.folder('C',18)/case/'reference-full-D-summary.json').read_text())
            self.assertEqual(summary['cases'],gate.domain(gate.CASES[case])['cases'])
            self.assertEqual(summary['reversed'],summary['cases'])
            self.assertTrue(summary['complete'])
            summary['shards'].pop()
            path=self.g.folder('C',18)/case/'reference-full-D-summary.json'
            path.write_text(json.dumps(summary))
            with self.assertRaises(gate.batch_reverse.BatchError):
                self.g.domain_summary('C',18,case,'reference')

    def test_failed_command_leaves_no_complete_aggregate(self):
        self.prepare('r1_lookup')
        def fail(tag,argv,**kw):raise gate.GateError('incomplete stream')
        self.g.command=fail
        with self.assertRaises(gate.GateError):self.g.exhaustive_D('C',18,'r1_lookup','reference')
        path=self.g.folder('C',18)/'r1_lookup/reference-full-D-summary.json'
        self.assertFalse(json.loads(path.read_text())['complete'])

    def test_capability_probe_is_cached_and_vm_legacy_route_is_retained(self):
        def help_command(tag,argv,**kwargs):
            self.calls.append((tag,argv,kwargs))
            return b'bennett-isa-ref reverse-action-batch OBJECT ACTION\n'
        self.g.command=help_command
        self.assertTrue(self.g.batch_available('reference'))
        self.assertTrue(self.g.batch_available('reference'))
        self.assertEqual(len(self.calls),1)
        self.g.batch_available=lambda name:False
        self.g.command=lambda tag,argv,**kw:self.calls.append((tag,argv,kw)) or b''
        self.g.exhaustive_D('C',18,'r1_lookup','VM')
        self.assertNotIn('--range',self.calls[-1][1])
        with self.assertRaises(gate.GateError):self.g.exhaustive_D('C',18,'r1_lookup','reference')


if __name__=='__main__':unittest.main()
