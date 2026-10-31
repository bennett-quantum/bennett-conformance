#!/usr/bin/env python3
"""Exercise the gate's reversal checks using real published control outputs.

These controls validate driver glue, never count as hashtable acceptance.
"""
import argparse
import copy
import json
from pathlib import Path
import subprocess
import unittest
import tempfile
import gate


class DriverChecks(unittest.TestCase):
    def test_fixture_route_is_only_the_documented_additions(self):
        f = gate.HERE / 'fixtures'
        self.assertEqual(gate.marked((f/'hashtable.c').read_text()),
                         (f/'hashtable_marked.c').read_text())

    def test_real_reversal_outputs_and_corrupted_state_are_distinguished(self):
        for name, value in outputs.items():
            with self.subTest(executor=name, mutation='none'):
                self.assertEqual(value['status'], 'REVERSED')
                gate.Gate.restored(value)
            for key, replacement in [('equals_entry',False), ('memory_equals_entry',False),
                                     ('first_difference','byte at heap+1'),
                                     ('memory_digest','corrupted digest')]:
                bad = copy.deepcopy(value)
                bad[key] = replacement
                with self.subTest(executor=name, mutation=key):
                    with self.assertRaises(AssertionError):
                        gate.Gate.restored(bad)

    def test_known_failure_stays_red_and_blocks_dependants(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = argparse.Namespace(output=tmp)
            g = gate.Gate(args)
            def failure():
                raise gate.GateError('18/production: exit 124\n\ntimeout\n')
            g.stage(18, 'production', failure)
            g.stage(18, 'forward', lambda: self.fail('blocked consumer executed'), ('production',))
            self.assertEqual(g.rows[18]['production']['status'], 'FAIL')
            self.assertEqual(g.rows[18]['forward']['status'], 'BLOCKED')
            self.assertEqual(g.rows[18]['forward']['beads'], ['bp-au6'])

    def test_missing_runtime_never_falls_back_to_corpus_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            args = argparse.Namespace(output=tmp, runtimes18=str(Path(tmp)/'missing.bc'))
            g = gate.Gate(args)
            g.stage(18, 'runtime link', lambda: g.runtime_link(18))
            self.assertEqual(g.rows[18]['runtime link']['status'], 'FAIL')
            self.assertEqual(g.rows[18]['runtime link']['beads'], ['brt-3sa'])
            self.assertEqual(g.manifest['commands'], [])

    def test_bad_results_and_host_calls_never_equal_native(self):
        # Boundary encodings use the public integer-result JSON contract.
        base = {'fault':None,'faults':[],'host_requests':[],
                'results':[{'type':'i64','value':'18446744073709551615'}]}
        self.assertEqual(gate.Gate.result(base), -1)
        for key, replacement in [('fault',4), ('faults',[{'code':7}]),
                                 ('host_requests',[{'service':'malloc'}]),
                                 ('results',[])]:
            bad = copy.deepcopy(base)
            bad[key] = replacement
            with self.subTest(mutation=key):
                with self.assertRaises(AssertionError):
                    gate.Gate.result(bad)


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--vm', required=True, type=Path, help='detached-worktree VM executable')
    p.add_argument('--reference', type=Path, default=gate.WORKSPACE/'bennett-isa-spec-review/reference-interpreter/target/debug/bennett-isa-ref')
    args = p.parse_args()
    fixture = gate.WORKSPACE/'bennett-isa-spec-review/reference-interpreter/tests/fixtures/registers.bisa'
    outputs = {}
    for name, exe in [('reference',args.reference),('VM',args.vm)]:
        outputs[name] = json.loads(subprocess.check_output([str(exe),'run-program',str(fixture),'allocation','--reverse']))
    unittest.main(argv=['check_driver.py'])
