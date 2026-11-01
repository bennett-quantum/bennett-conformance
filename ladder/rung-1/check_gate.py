"""Acceptance glue: deliberate corruption must stay red; no synthetic corpus passes."""
import argparse
import copy
import json
import os
from pathlib import Path
import tempfile
import unittest
import gate

class Checks(unittest.TestCase):
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
    def test_missing_toolchain_is_failure(self):
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as tmp:
            g=gate.Gate(argparse.Namespace(output=tmp,timeout=300))
            g.stage('C/18','toolchain',lambda:g.command('missing',['/nonexistent/rung1-clang','--version']))
            self.assertEqual(g.rows['C/18']['toolchain']['status'],'FAIL')
            self.assertEqual(g.manifest['commands'][-1]['exit'],127)

if __name__=='__main__': unittest.main()
