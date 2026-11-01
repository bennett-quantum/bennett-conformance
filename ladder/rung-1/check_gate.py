"""Acceptance glue: deliberate corruption must stay red; no synthetic corpus passes."""
import argparse
from contextlib import redirect_stdout
import io
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
import gate

class Checks(unittest.TestCase):
    @staticmethod
    def bitcode_header(version):
        producer='LLVM'+version+'-stock'
        return ('<IDENTIFICATION_BLOCK_ID NumWords=14>\n  <STRING '+
                ' '.join(f'op{i}={ord(c)}' for i,c in enumerate(producer))+
                '/>\n</IDENTIFICATION_BLOCK_ID>').encode()

    def test_frontend_IR_applicability(self):
        row=gate.frontend_applicability(self.bitcode_header('23.1.1'),18)
        self.assertEqual(row['status'],'N/A')
        self.assertEqual(gate.cell(row),'N/A · §19.8 (frontend IR 23.1.1 newer than LLVM 18)')
        self.assertEqual(row['beads'],[])
        for version in ('18.1.3','17.0.6'):
            with self.subTest(version=version):
                self.assertIsNone(gate.frontend_applicability(self.bitcode_header(version),18))
        self.assertIsNone(gate.frontend_applicability(self.bitcode_header('20.1.8'),23))
        with self.assertRaises(gate.GateError):gate.frontend_applicability(b'no producer',18)

    def test_NA_ignored_by_exit_and_preserved_by_classification(self):
        from classify_report import classify, table
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as tmp:
            g=gate.Gate(argparse.Namespace(output=tmp,timeout=300))
            row=gate.frontend_applicability(self.bitcode_header('23.1.1'),18)
            for key in g.rows:
                g.rows[key]={s:dict(row) for s in gate.STAGES}
                g.children[key]={c:copy.deepcopy(g.rows[key]) for c in gate.CASES}
            with redirect_stdout(io.StringIO()):self.assertEqual(g.finish(),0)
            report=json.loads((g.run/'report.json').read_text())
            self.assertEqual(classify(report,[]),report)
            self.assertIn(gate.cell(row),table(report))
            g.rows['C/18']['pipeline']={'status':'BLOCKED','beads':['real'], 'error':'blocked'}
            with redirect_stdout(io.StringIO()):self.assertEqual(g.finish(),1)

    def test_input_product_and_full_D(self):
        self.assertEqual(len(list(gate.inputs(1))),256)
        self.assertEqual(len(list(gate.inputs(2))),65536)
        d=gate.domain(2)
        self.assertEqual(d['cases'],65536*256*16)
        self.assertEqual(d['dirty_bits'],0)
        self.assertTrue(d['input_preservation'])
    def test_bad_native_counts(self):
        with self.assertRaises(AssertionError): gate.split_native(b'')
        values=gate.split_native(bytes(gate.NATIVE_COUNT))
        self.assertEqual(len(values['r1_gcd']),65536)
    def test_fault_and_reverse_corruption(self):
        value={'payload':255,'code':0,'equals_entry':True,'frames_clean':True,
               'first_difference':None,'memory_equals_entry':True,
               'entry_memory_digest':'a','memory_digest':'a','status':'REVERSED'}
        self.assertEqual(gate.result(value),255)
        gate.restored(value)
        for key,bad in [('code',4),('equals_entry',False),('frames_clean',False),
                        ('first_difference','input changed'),('memory_digest','b')]:
            v=copy.deepcopy(value);v[key]=bad
            with self.subTest(key=key),self.assertRaises(AssertionError):
                gate.result(v);gate.restored(v)
    def test_red_dependencies(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as tmp:
            g=gate.Gate(argparse.Namespace(output=tmp,timeout=300))
            g.known=[{'id':'test', 'pattern':'missing toolchain'}]
            def fail(): raise gate.GateError('missing toolchain')
            g.stage('C/18','frontend',fail)
            g.stage('C/18','pipeline',lambda:self.fail('ran blocked stage'),('frontend',))
            self.assertEqual(g.rows['C/18']['pipeline']['beads'],['test'])
            self.assertEqual(g.rows['C/18']['pipeline']['status'],'BLOCKED')
    def test_full_D_rejects_changed_inputs_target_fault_phase_workspace(self):
        # One input bit pattern, arbitrary target/fault; exact sparse basis state.
        before='10000000101010000000'
        after='00000000101010000000'
        v={'forward_state':{after:[1,0]},'state':{before:[1,0]},'wires':20,
           'status':'REVERSED','equals_entry':True,'first_difference':None,'in':[1],'mut':[1,5]}
        gate.check_action(v,(1,),1,5,1)
        for k,bad in [('forward_state',{before:[1,0]}),('state',{after:[1,0]}),
                      ('wires',21),('in',[0]),('mut',[1,4]),('forward_state',{after:[-1,0]})]:
            w=copy.deepcopy(v);w[k]=bad
            with self.subTest(key=k),self.assertRaises(AssertionError):gate.check_action(w,(1,),1,5,1)
    def test_classification_preserves_red_and_propagates_new_beads(self):
        from classify_report import classify
        rows={'source':{'status':'FAIL','beads':['UNFILED'],'error':'measured refusal'},
              'next':{'status':'BLOCKED','beads':['UNFILED'],'error':'Not run; requires source'}}
        r={'stages':{'C/18':copy.deepcopy(rows)},'cases':{'C/18':{'f':copy.deepcopy(rows)}}}
        got=classify(r,[{'id':'real-bead','pattern':'measured refusal'}])
        self.assertEqual(got['stages']['C/18']['next']['beads'],['real-bead'])
        self.assertEqual(got['stages']['C/18']['source']['status'],'FAIL')
        self.assertEqual(r['stages']['C/18']['source']['beads'],['UNFILED'])
    def test_adapter_probe_checks_reply_not_only_exit(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as tmp:
            g=gate.Gate(argparse.Namespace(output=tmp,timeout=300))
            d=g.folder('C',18);(d/'r1_popcount/out').mkdir(parents=True)
            (d/'r1_popcount/out/full-domain.bennett').write_text('already admitted test adapter')
            (d/'r1_popcount-native.bin').write_bytes(bytes(256))
            g.executor=lambda name:Path('/test-executor')
            g.command=lambda *a,**kw:b'{"forward_state":{}}'
            with self.assertRaisesRegex(AssertionError,'entire'):
                g.full_domain_interface('C',18,'r1_popcount','reference')
    def test_missing_toolchain_is_failure(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as tmp:
            g=gate.Gate(argparse.Namespace(output=tmp,timeout=300))
            g.stage('C/18','toolchain',lambda:g.command('missing',['/nonexistent/rung1-clang','--version']))
            self.assertEqual(g.rows['C/18']['toolchain']['status'],'FAIL')
            self.assertEqual(g.manifest['commands'][-1]['exit'],127)

if __name__=='__main__': unittest.main()
