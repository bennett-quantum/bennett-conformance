#!/usr/bin/env python3
"""Gate glue tests; synthetic results never count as corpus acceptance."""
import argparse
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import gate

class GateChecks(unittest.TestCase):
    def new(self, tmp):
        return gate.Gate(argparse.Namespace(output=tmp, timeout=300))

    def test_plain_fixture_and_valid_clause_tags(self):
        g = self.new(self.tmp)
        g.fixtures()
        self.assertEqual((gate.HERE/'fixtures/hello.c').read_text(),
                         '#include <stdio.h>\n\nint main(void) {\n    printf("hello, world\\n");\n    return 0;\n}\n')

    def test_known_failure_remains_red_and_blocks(self):
        g = self.new(self.tmp)
        g.known = [{'id':'test-bead', 'pattern':'missing musl'}]
        def fail():
            raise gate.GateError('missing musl')
        g.stage(18, 'source', fail)
        g.stage(18, 'consumer', lambda:self.fail('ran blocked consumer'), ('source',))
        self.assertEqual(g.rows[18]['source']['status'], 'FAIL')
        self.assertEqual(g.rows[18]['consumer']['beads'], ['test-bead'])
        self.assertEqual(g.rows[18]['consumer']['status'], 'BLOCKED')

    def test_frontend_never_uses_host_headers(self):
        g = self.new(self.tmp)
        g.sysroots = {18:Path(self.tmp)/'musl'}
        (g.sysroots[18]/'include').mkdir(parents=True)
        (g.sysroots[18]/'include/stdio.h').write_text('/* test header */')
        commands = []
        g.command = lambda tag, args, **kw:commands.append(list(map(str,args))) or b''
        g.frontend(18)
        a = commands[0]
        self.assertIn('-nostdinc', a)
        self.assertIn('--sysroot='+str(g.sysroots[18]), a)
        self.assertIn(str(g.sysroots[18]/'include'), a)
        self.assertNotIn('-ffreestanding', a)

    def test_native_comparison_forbids_host_libc(self):
        g = self.new(self.tmp)
        commands=[]
        g.command=lambda tag, argv, **kw: commands.append(list(map(str,argv))) or b'hello, world\n'
        d=g.run/'18'; (d/'out').mkdir(parents=True)
        (d/'out/native-comparison.json').write_text('{}')
        g.runtime_files=lambda m:(Path('/runtime.bc'),Path('/arith.bc'))
        g.driver_exe=Path('/driver')
        # No comparison manifest may silently fall back to clang hello.c.
        with self.assertRaises((gate.GateError, KeyError, AssertionError)):
            g.native(18)
        self.assertEqual(len(commands), 1)
        self.assertEqual(commands[0][0], '/driver')

    def test_native_link_rejects_host_libc_archive(self):
        g=self.new(self.tmp)
        d=g.run/'18'; out=d/'out'; out.mkdir()
        (out/'device.bc').write_bytes(b'materialised')
        rt=Path(self.tmp)/'runtime.bc'; ar=Path(self.tmp)/'arith.bc'
        rt.write_bytes(b'runtime'); ar.write_bytes(b'arith')
        g.publications={'runtime':Path(self.tmp)}
        g.runtime_files=lambda m:(rt,ar)
        g.driver_exe=Path('/driver')
        g.command=lambda *a,**kw:b''
        argv=[g.clang(18),'-nostdlib','-nostdinc','-static','--sysroot='+str(g.sysroots[18]),
              str(out/'device.bc'),str(rt),str(ar),'/usr/lib/libc.a','-o',str(d/'native')]
        (out/'native-comparison.json').write_text(json.dumps({'llvm_major':18,
            'materialised_sha256':gate.sha(out/'device.bc'),'link_argv':argv}))
        with self.assertRaisesRegex(AssertionError,'unapproved comparison input'):
            g.native(18)

    def test_command_evidence_records_nonzero_exit_and_timeout(self):
        g=self.new(self.tmp)
        import sys
        with self.assertRaises(gate.GateError):
            g.command('test/exit',[sys.executable,'-c','import sys; print("evidence"); sys.exit(3)'])
        self.assertEqual(g.manifest['commands'][-1]['exit'],3)
        self.assertEqual((g.run/'test/exit.stdout').read_bytes(),b'evidence\n')
        with self.assertRaises(gate.GateError):
            g.command('test/timeout',[sys.executable,'-c','import time; time.sleep(5)'],timeout=0.01)
        self.assertEqual(g.manifest['commands'][-1]['exit'],124)

    def test_reset_includes_console_and_tape(self):
        value={'status':'REVERSED','fault':None,'faults':[], 'host_requests':[],
               'results':[{'type':'i32','value':'0'}], 'equals_entry':True,
               'memory_equals_entry':True,'first_difference':None,
               'memory_digest':'a','entry_memory_digest':'a','console_hex':'',
               'effect_tape_length':0, 'tape':[{'dev':'console','op':'write','args':[4096,13],
                                            'bytes':b'hello, world\n'.hex()}]}
        gate.Gate.restored(value)
        for key,bad in [('equals_entry',False),('memory_digest','b'),
                        ('console_hex','68'),('effect_tape_length',1),
                        ('first_difference','console')]:
            v=copy.deepcopy(value); v[key]=bad
            with self.subTest(key=key), self.assertRaises(AssertionError):
                gate.Gate.restored(v)

    def test_replay_consumes_saved_tape_and_detects_tampering(self):
        g = self.new(self.tmp)
        value={'status':'COMPLETED','fault':None,'faults':[], 'host_requests':[],
               'results':[{'type':'i32','value':'0'}], 'console_hex':b'hello, world\n'.hex(),
               'tape':[{'dev':'console','op':'write','args':[4096,13], 'bytes':b'hello, world\n'.hex()}],
               'effect_tape_length':1}
        g.forward_results={(18,'reference'):value}
        g.argv_replies=lambda m:Path('/argv-replies.json')
        calls=[]
        g.executor=lambda name:Path('/executor')
        def command(tag, argv, **kw):
            calls.append(list(map(str,argv)))
            if 'tampered' in tag:
                raise gate.GateError('replay tape mismatch')
            return json.dumps(value).encode()
        g.command=command
        g.replay(18,'reference')
        self.assertEqual(len(calls),2)
        self.assertTrue(all('--replay-tape' in c for c in calls))
        g.command=lambda *a,**kw:json.dumps(value).encode()
        with self.assertRaisesRegex(AssertionError,'accepted a tampered'):
            g.replay(18,'reference')

    def setUp(self):
        self.directory=tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.tmp=self.directory.name
    def tearDown(self):
        self.directory.cleanup()

if __name__=='__main__':
    unittest.main()
