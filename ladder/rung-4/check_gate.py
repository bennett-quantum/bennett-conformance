#!/usr/bin/env python3
"""Rung-4 gate glue tests: synthetic values never count as acceptance."""
import argparse
import cmath
import json
import math
import os
from pathlib import Path
import re
import tempfile
import unittest
from unittest.mock import Mock, patch
import gate

N256_MATCHES = [50, 57, 146, 164, 210, 250]


def args(tmp):
    return argparse.Namespace(output=tmp, timeout=300, jobs=2, production_timeout=300,
                              sweep_timeout=300, reference='/nonexistent', record=False)


class Independent(unittest.TestCase):
    def test_predicate_reproduces_spike_counts(self):
        # REPORT.md §1.1: 6 of 256, 13 of 1024, 11,698 of 2^20.
        self.assertEqual(sum(gate.truth(256, 8)), 6)
        self.assertEqual(sum(gate.truth(1024, 10)), 13)
        self.assertEqual(sum(gate.truth(1 << 20, 20)), 11698)

    def test_truth_table_is_zero_beyond_n(self):
        t = gate.truth(1000, 10)
        self.assertEqual(len(t), 1024)
        self.assertEqual(t[1000:], [0] * 24)
        self.assertEqual([x for x, b in enumerate(t) if b], [50, 57, 146, 164, 210, 250, 524, 576, 579, 591, 845, 872, 970])
        self.assertEqual([x for x, b in enumerate(gate.truth(256, 8)) if b], N256_MATCHES)

    def test_sizes_and_exponents(self):
        self.assertEqual(gate.bits_for(1000), 10)
        self.assertEqual(gate.bits_for(1024), 10)
        self.assertEqual(gate.region_exponent(16384), 14)
        self.assertEqual(gate.region_exponent(16385), 15)
        self.assertIn(1 << 10, gate.ORACLE_SIZES)
        self.assertIn(1 << 8, gate.GROVER_SIZES)
        self.assertEqual(gate.ESTIMATE_SIZE, 1 << 20)
        self.assertTrue(all(n <= 1 << 10 for n in gate.ORACLE_SIZES))
        self.assertTrue(all(n <= 1 << 8 for n in gate.GROVER_SIZES))


class GroverPlan(unittest.TestCase):
    def test_iterations_and_probability(self):
        p = gate.grover_plan(256, 8)
        self.assertEqual((p['M'], p['k']), (6, 5))
        self.assertAlmostEqual(p['success'], 0.98570, places=4)
        p = gate.grover_plan(1 << 20, 20)
        self.assertEqual((p['M'], p['k']), (11698, 7))

    def test_no_match_control_stays_uniform(self):
        p = gate.grover_plan(16, 4)
        self.assertEqual((p['M'], p['k'], p['success']), (0, 1, 0.0))
        s = gate.simulate(gate.truth(16, 4), 1)
        for a in s:
            self.assertAlmostEqual(abs(a), 1 / 4)

    def test_simulation_matches_closed_form_for_each_k(self):
        t = gate.truth(256, 8)
        theta = math.asin(math.sqrt(6 / 256))
        for k in range(0, 8):
            s = gate.simulate(t, k)
            got = sum(abs(a) ** 2 for a, b in zip(s, t) if b)
            self.assertAlmostEqual(got, math.sin((2 * k + 1) * theta) ** 2, places=12)
            self.assertAlmostEqual(sum(abs(a) ** 2 for a in s), 1.0, places=12)


def dump(n, truth, k, phase=1, a=4, image='00', corrupt=None):
    s = gate.simulate(truth, k)
    amps = []
    for x, v in enumerate(s):
        for t, sign in ((0, 1), (1, -1)):
            z = phase * v * sign / math.sqrt(2)
            amps.append({'basis': {'1': str(x), '2': str(t), '3': '0', '4': '0'}, 're': z.real, 'im': z.imag})
    d = {'registers': [{'reg': 0, 'width': 8 << a}, {'reg': 1, 'width': n}, {'reg': 2, 'width': 1},
                       {'reg': 3, 'width': 4}, {'reg': 4, 'width': n}],
         'constant': {'0': image}, 'amps': amps}
    if corrupt:
        corrupt(d)
    return d


class StateDump(unittest.TestCase):
    def setUp(self):
        self.truth = gate.truth(64, 6)

    def check(self, d):
        return gate.check_state(d, 6, self.truth, 4, 4, '00', 2)

    def test_accepts_any_global_phase(self):
        for phase in (1, -1, 1j, cmath.exp(0.7j)):
            r = self.check(dump(6, self.truth, 4, phase))
            self.assertLess(r['max_amplitude_error'], 1e-12)
            self.assertAlmostEqual(r['success'], gate.grover_plan(64, 6)['success'], places=12)

    def test_rejects_non_pivot_nan_amplitude(self):
        d = dump(6, self.truth, 4)
        pivot = max(range(len(self.truth)), key=lambda x: abs(gate.simulate(self.truth, 4)[x]))
        row = next(r for r in d['amps'] if int(r['basis']['1']) != pivot)
        row['re'] = float('nan')
        d = json.loads(json.dumps(d))  # Reproduce the executor dump ingestion in the review.
        with self.assertRaisesRegex(AssertionError, 'non-finite amplitude'):
            self.check(d)

    def test_rejects_non_finite_amplitudes_on_every_branch(self):
        for i in range(2 * len(self.truth)):
            for component in ('re', 'im'):
                for value in (math.nan, math.inf, -math.inf):
                    with self.subTest(row=i, component=component, value=value):
                        d = dump(6, self.truth, 4)
                        d['amps'][i][component] = value
                        with self.assertRaisesRegex(AssertionError, 'non-finite amplitude'):
                            self.check(d)

    def test_rejects_finite_amplitude_that_overflows_phase(self):
        d = dump(6, self.truth, 4)
        pivot = max(range(len(self.truth)), key=lambda x: abs(gate.simulate(self.truth, 4)[x]))
        row = next(r for r in d['amps'] if (int(r['basis']['1']), int(r['basis']['2'])) == (pivot, 0))
        row['re'] = 1e308
        with self.assertRaisesRegex(AssertionError, 'amplitude'):
            self.check(d)

    def test_rejects_wrong_iteration_count(self):
        with self.assertRaisesRegex(AssertionError, 'amplitude'):
            self.check(dump(6, self.truth, 3))

    def test_rejects_wrong_marked_set(self):
        wrong = list(self.truth)
        wrong[0] = 1
        with self.assertRaisesRegex(AssertionError, 'amplitude'):
            self.check(dump(6, wrong, 4))

    def test_rejects_garbage_fault_and_ancilla_and_target(self):
        for reg, value in (('3', '1'), ('4', '1')):
            def corrupt(d, reg=reg, value=value):
                d['amps'][0]['basis'][reg] = value
            with self.subTest(reg=reg), self.assertRaises(AssertionError):
                self.check(dump(6, self.truth, 4, corrupt=corrupt))

    def test_rejects_database_register_change(self):
        with self.assertRaisesRegex(AssertionError, 'database'):
            self.check(dump(6, self.truth, 4, image='01'))

    def test_rejects_database_entangled_with_branch(self):
        def corrupt(d):
            d['amps'][0]['basis']['0'] = '01'
        with self.assertRaisesRegex(AssertionError, 'database'):
            self.check(dump(6, self.truth, 4, corrupt=corrupt))

    def test_rejects_unnormalised_and_truncated(self):
        def drop(d):
            d['amps'].pop()
        with self.assertRaises(AssertionError):
            self.check(dump(6, self.truth, 4, corrupt=drop))


class StateIngestion(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.d.cleanup)
        self.g = object.__new__(gate.Gate)
        self.g.args = args(self.d.name)
        self.g.run = Path(self.d.name)
        out = self.g.run / '18/N64'
        out.mkdir(parents=True)
        self.state = out / 'reference-state.json'
        launch = out/'out/rung4_grover'
        launch.mkdir(parents=True)
        (launch/'build.json').write_text(json.dumps({'budgets': [{'steps': '37'}, {'steps': '11'}, {'steps': '0'}, {'steps': '37'}]}))
        image = out / 'image.bin'
        image.write_bytes(bytes(16))
        self.g.images = {18: {64: image}}
        self.g.tables = {64: gate.truth(64, 6)}
        self.g.plans = {64: gate.grover_plan(64, 6)}
        self.g.executor = Mock(return_value='/fake-executor')
        self.g.command = Mock(return_value=b'{"status":"COMPLETED","fault":null,"faults":[],"host_requests":[]}')
        self.dump = dump(6, self.g.tables[64], 4, image=bytes(16).hex())

    def test_accepts_finite_state_json(self):
        self.state.write_text(json.dumps(self.dump))
        result, plan = self.g.grover_state(18, 'reference', 64)
        self.assertLess(result['max_amplitude_error'], 1e-12)
        self.assertAlmostEqual(result['success'], plan['success'], places=12)

    def test_state_argv_includes_each_declared_allowance_in_order(self):
        self.state.write_text(json.dumps(self.dump))
        self.g.grover_state(18, 'reference', 64)
        argv = list(map(str, self.g.command.call_args.args[1]))
        self.assertIn('--args', argv)
        self.assertEqual(argv[argv.index('--args')+1], '37,11,0,37')

    def test_rejects_non_finite_json_numbers_anywhere(self):
        for token in ('NaN', 'Infinity', '-Infinity', '1e400', '-1e400'):
            for location in ('amplitude', 'extra field'):
                with self.subTest(token=token, location=location):
                    bad = json.loads(json.dumps(self.dump))
                    if location == 'amplitude':
                        bad['amps'][0]['re'] = 'NONFINITE'
                    else:
                        bad['extra'] = ['NONFINITE']
                    self.state.write_text(json.dumps(bad).replace('"NONFINITE"', token))
                    with self.assertRaisesRegex(ValueError, 'non-finite JSON number'):
                        self.g.grover_state(18, 'reference', 64)


class LaunchArguments(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.d.cleanup)
        self.g = gate.Gate(args(self.d.name))
        self.g.executor = Mock(return_value='/executor')
        self.g.tables = {16: [0]*16}
        self.g.plans = {16: gate.grover_plan(16)}
        self.g.sweep = lambda tag, items, action: [action(item) for item in items]
        self.budgets = [{'steps': '37'}, {'steps': '11'}, {'steps': '0'}, {'steps': '37'}]
        for name in ('rung4_tuple', 'rung4_grover'):
            launch = self.g.run/'18/N16/out'/name
            launch.mkdir(parents=True)
            (launch/'build.json').write_text(json.dumps({'budgets': self.budgets}))

    def test_allowances_require_the_launch_build_metadata(self):
        launch = self.g.run/'18/N16/out/rung4_tuple'
        (launch/'build.json').unlink()
        with self.assertRaisesRegex(gate.GateError, 'launch budget allowances.*build.json'):
            gate.launch_allowances(launch)

    def test_allowances_reject_missing_or_malformed_budget_entries(self):
        launch = self.g.run/'18/N16/out/rung4_tuple'
        for metadata in ({}, {'budgets': None}, {'budgets': {}},
                         {'budgets': [{}]}, {'budgets': [None]},
                         *({'budgets': [{'steps': value}]} for value in (None, True, -1, 1.5, '01', '+1', ''))):
            with self.subTest(metadata=metadata):
                (launch/'build.json').write_text(json.dumps(metadata))
                with self.assertRaisesRegex(gate.GateError, 'budget.*build.json'):
                    gate.launch_allowances(launch)
        (launch/'build.json').write_text('{')
        with self.assertRaisesRegex(gate.GateError, 'launch budget allowances.*build.json'):
            gate.launch_allowances(launch)

    def test_explicit_empty_budget_array_supplies_zero_allowances(self):
        launch = self.g.run/'18/N16/out/rung4_tuple'
        (launch/'build.json').write_text(json.dumps({'budgets': []}))
        self.assertEqual(gate.launch_allowances(launch), ())

    def test_allowances_preserve_zero_repetitions_and_integer_values(self):
        launch = self.g.run/'18/N16/out/rung4_tuple'
        (launch/'build.json').write_text(json.dumps({'budgets': [{'steps': 0}, {'steps': '91'}, {'steps': 0}]}))
        self.assertEqual(gate.launch_allowances(launch), ('0', '91', '0'))

    def test_tuple_argv_appends_allowances_after_entry_arguments(self):
        def execute(argv):
            text = argv[argv.index('--args')+1].split(',')
            self.assertEqual(text[3:], ['37', '11', '0', '37'])
            x, t, e = map(int, text[:3])
            return {'status': 'COMPLETED', 'fault': None, 'faults': [], 'host_requests': [],
                    'run_state': 'CLOSED', 'results': [{'value': str(x | (t << 4) | (e << 5))}]}
        self.g.quiet = Mock(side_effect=execute)
        with patch.object(gate, 'ORACLE_SIZES', (16,)):
            self.g.tuples(18, 'reference')
        self.assertEqual(self.g.quiet.call_count, 16*2*16)

    def test_outcome_argv_passes_allowances_without_entry_arguments(self):
        def execute(tag, argv, **kwargs):
            self.assertIn('--args', argv)
            self.assertEqual(argv[argv.index('--args')+1], '37,11,0,37')
            # Uniform four-bit register: half draws select 1111; zero draws select 0000.
            expected = {'half': 15, 'zero': 0, 'select-0': 0, 'select-15': 15}
            return json.dumps({'status': 'COMPLETED', 'fault': None, 'faults': [], 'host_requests': [],
                               'results': [{'value': str(expected[tag.split('-grover-')[1]])}]}).encode()
        self.g.command = Mock(side_effect=execute)
        with patch.object(gate, 'GROVER_SIZES', (16,)):
            self.g.outcomes(18, 'reference')
        self.assertEqual(self.g.command.call_count, 4)


class ReducedMeasuredState(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(self.d.cleanup)
        self.g = gate.Gate(args(self.d.name))
        self.fixture = Path(self.d.name)/'isa/reference-interpreter/tests/fixtures/registers.bisa'
        self.fixture.parent.mkdir(parents=True)
        self.fixture.write_text('fn cc @measurement () (r:i2) () () ()\n^entry():\nq = qreg.new 2\nr = measure q\nret r\n')
        self.g.sources.paths['bennett-isa-spec'] = Path(self.d.name)/'isa'
        self.g.executor = Mock(return_value='/executor')

    def measured(self, tag, argv):
        self.assertEqual(argv[1:4], ['run-program', self.fixture, 'measurement'])
        self.assertNotIn('--state-out', argv)
        draw = argv[argv.index('--draws')+1]
        bits, value = {'0.0': ('00', 0), '0.9': ('11', 3)}[draw]
        return json.dumps({'status': 'COMPLETED', 'fault': None, 'faults': [], 'host_requests': [],
                           'run_state': 'CLOSED', 'results': [{'type': 'i2', 'value': str(value)}],
                           'measurements': [{'bits': bits, 'reg': 1, 'value': value}]}).encode()

    def test_reduced_state_comes_from_a_measuring_launch(self):
        self.g.command = Mock(side_effect=self.measured)
        self.g.state_out('VM')
        state = json.loads((self.g.run/'VM-probe-state.json').read_text())
        self.assertEqual([r['measurement']['bits'] for r in state['observations']], ['00', '11'])
        self.assertEqual(self.g.command.call_count, 2)
        self.assertEqual(state['fixture_sha256'], gate.sha(self.fixture))

    def test_unmeasured_or_wrong_state_is_rejected(self):
        for measurements in ([], [{'bits': '01', 'reg': 1, 'value': 1}]):
            with self.subTest(measurements=measurements):
                self.g.command = Mock(return_value=json.dumps({'status': 'COMPLETED', 'fault': None,
                    'faults': [], 'host_requests': [], 'run_state': 'CLOSED',
                    'results': [{'type': 'i2', 'value': '0'}], 'measurements': measurements}).encode())
                with self.assertRaisesRegex(AssertionError, 'measurement'):
                    self.g.state_out('reference')


class DrawPrediction(unittest.TestCase):
    def test_follows_isa_cdf_rule(self):
        # Wire 0 is measured first: outcome 0 first in lexical order, strict > on the CDF.
        probs = [0.5, 0.25, 0.25, 0.0]   # x = 0, 1, 2, 3, wire 0 = low bit
        # P(wire0=0) = p[0] + p[2] = 0.75.
        self.assertEqual(gate.predict_outcome(probs, 2, [0.0, 0.0]), 0)
        self.assertEqual(gate.predict_outcome(probs, 2, [0.74, 0.0]), 0)
        # Exact tie selects the next nonzero-weight outcome.
        self.assertEqual(gate.predict_outcome(probs, 2, [0.75, 0.0]), 1)
        self.assertEqual(gate.predict_outcome(probs, 2, [0.74, 0.9]), 2)

    def test_draws_for_selects_exactly_the_chosen_branch(self):
        s = gate.simulate(gate.truth(256, 8), 5)
        probs = [abs(a) ** 2 for a in s]
        for x in (N256_MATCHES[0], N256_MATCHES[-1], 0, 255):
            self.assertEqual(gate.predict_outcome(probs, 8, gate.draws_for(probs, 8, x)), x)


class Tuples(unittest.TestCase):
    def test_packing_and_domain(self):
        self.assertEqual(gate.domain_size(10), 32768)
        self.assertEqual(gate.expected_tuple(3, 1, 7, 1, 8), 3 | (0 << 8) | (7 << 9))
        self.assertEqual(gate.expected_tuple(3, 0, 7, 1, 8), 3 | (1 << 8) | (7 << 9))
        self.assertEqual(len(list(gate.domain(4))), 512)

    def test_tuple_failure_modes(self):
        good = {'status': 'COMPLETED', 'fault': None, 'faults': [], 'host_requests': [],
                'run_state': 'CLOSED', 'results': [{'type': 'i64', 'value': str(5)}]}
        self.assertEqual(gate.tuple_result(good), 5)
        for key, bad in (('status', 'FAILED'), ('fault', 4), ('faults', [4]),
                         ('host_requests', [{'service': 'x'}]), ('run_state', 'OPEN')):
            with self.subTest(key=key), self.assertRaises(AssertionError):
                gate.tuple_result({**good, key: bad})

    def test_oracle_run_requires_zero_fault_and_native_payload(self):
        gate.oracle_row({'code': 0, 'payload': 1, 'steps': 7}, 1, 100)
        for row, want, budget in (({'code': 4, 'payload': 0, 'steps': 7}, 0, 100),
                                  ({'code': 0, 'payload': 0, 'steps': 7}, 1, 100),
                                  ({'code': 0, 'payload': 0, 'steps': 101}, 0, 100)):
            with self.subTest(row=row), self.assertRaises(AssertionError):
                gate.oracle_row(row, want, budget)

    def test_step_check_must_be_exhaustive_over_fixed_image(self):
        good = {'cases': 256, 'fixed_regions': ['m0'], 'input_and_memory_bits': 8,
                'mode': 'exhaustive', 'status': 'ok'}
        gate.step_report(good, 8)
        for key, bad in (('cases', 255), ('fixed_regions', []), ('input_and_memory_bits', 9),
                         ('mode', 'sampled'), ('status', 'failed')):
            with self.subTest(key=key), self.assertRaises(AssertionError):
                gate.step_report({**good, key: bad}, 8)


class StandIns(unittest.TestCase):
    """No reduced stand-in may pass for the rung."""

    def test_query_text_is_pinned(self):
        h = (gate.HERE / 'fixtures/rung4.h').read_text()
        self.assertIn("name LIKE '%an%'", h)
        self.assertIn('rowid = ?1', h)
        o = (gate.HERE / 'fixtures/oracle.c').read_text()
        for needle in ('sqlite3_deserialize', 'sqlite3_prepare_v2', 'sqlite3_bind_int64', 'sqlite3_step', 'SQLITE_DESERIALIZE_READONLY'):
            self.assertIn(needle, o)

    def test_oracle_interface_shape_is_required(self):
        good = 'oracle @ref (x0:wire[8] in) (tgt:wire[1]) (fault:wire[4]) (m0:region[14,8] in) (a0:i64)\n' + 'lproc @L1 ()\n' * 600
        gate.check_reference_object(good, 8, 14)
        for bad in (good.replace('wire[8]', 'wire[3]'),
                    good.replace('region[14,8]', 'region[4,8]'),
                    good.replace(' (m0:region[14,8] in)', ''),
                    'oracle @ref (x0:wire[8] in) (tgt:wire[1]) (fault:wire[4]) (m0:region[14,8] in) (a0:i64)\nlproc @L1 ()\n'):
            with self.subTest(bad=bad[:50]), self.assertRaises(AssertionError):
                gate.check_reference_object(bad, 8, 14)

    def test_program_must_contain_real_sqlite(self):
        ok = ''.join(f'define internal void @{s}(i32 %x) {{\n' for s in gate.SQLITE_WITNESSES)
        gate.check_program_text(ok)
        with self.assertRaisesRegex(AssertionError, 'memdbRead'):
            gate.check_program_text(ok.replace('@memdbRead', '@other'))

    def test_closure_audit_flags_unresolved_libc_and_ignores_markers(self):
        text = 'declare i32 @strncmp(ptr, ptr, i64)\ndeclare void @llvm.trap()\ndeclare ptr @__bennett_qalloc(i64, i64)\ndefine i32 @f() {\n'
        self.assertEqual(gate.unresolved(text), ['strncmp'])

    def test_coverage_rejects_trapped_lines(self):
        gate.check_coverage({'rows': [{'name': 'f', 'status': 'lifted'}]})
        with self.assertRaisesRegex(AssertionError, 'trapped'):
            gate.check_coverage({'rows': [{'name': 'localtime', 'status': 'trapped'}]})

    def test_sqlite_hash_mismatch_fails(self):
        g = gate.Gate(args(self.tmp))
        root = Path(self.tmp) / 'sqlite'
        root.mkdir()
        (root / 'sqlite3.c').write_text('/* edited */')
        (root / 'sqlite3.h').write_text('/* edited */')
        with self.assertRaisesRegex(AssertionError, 'unmodified'):
            g.check_sqlite_tree(root)

    def test_certificate_may_not_claim_proof_for_fixed_region(self):
        gate.check_certificate_verdict({'verdict': 'evidence', 'elements': 512, 'covered_fraction': '512/' + str(512 * 2 ** (8 * 16))}, 512, 4)
        for bad in ({'verdict': 'proof', 'elements': 512, 'covered_fraction': '1/1'},
                    {'verdict': 'evidence', 'elements': 511, 'covered_fraction': '511/' + str(512 * 2 ** (8 * 16))},
                    {'verdict': 'unchecked', 'elements': 0}):
            with self.subTest(bad=bad), self.assertRaises(AssertionError):
                gate.check_certificate_verdict(bad, 512, 4)

    def test_estimate_must_report_budget_padding_and_real_size(self):
        b = gate.PINS['budget']
        good = {'target': 'superconducting-grid-illustrative', 'label': 'bound',
                'budget': {'steps': b['steps'], 'heap': b['heap'], 'stack': b['stack'], 'depth': b['depth']},
                'padded': {'toffoli': '1'}, 'object_sha256': 'x'}
        gate.check_estimate(good, 'x')
        for key, bad in (('target', ''), ('label', None), ('object_sha256', 'y'),
                         ('padded', {}), ('budget', {'steps': 1})):
            with self.subTest(key=key), self.assertRaises(AssertionError):
                gate.check_estimate({**good, key: bad}, 'x')

    def setUp(self):
        self.d = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.tmp = self.d.name

    def tearDown(self):
        self.d.cleanup()


class Blockers(unittest.TestCase):
    def test_every_blocker_names_a_sentence_witness_and_fix(self):
        rows = json.loads((gate.HERE / 'blockers.json').read_text())
        self.assertEqual(len({r['id'] for r in rows}), len(rows))
        for r in rows:
            re.compile(r['pattern'])
            self.assertIn('Fixed means', r['determining_spec_and_command_and_fixed_means'])
            self.assertIn('witness', r['determining_spec_and_command_and_fixed_means'].lower())

    def test_measured_errors_map_to_their_beads(self):
        rows = json.loads((gate.HERE / 'blockers.json').read_text())
        cases = {'runtime libc closure incomplete, undefined after link: bcmp': 'brt-dj7',
                 '18/16-production: exit 1\nbennett: unimplemented: §5 PT10 composite operation (milestone 1) at %8845 = getelementptr inbounds %struct.sqlite3_value, ptr %246, <2 x i64> %8843': 'bp-1bb',
                 'bennett: unimplemented: §5.2 PT6 budget exhausted at AV(%407) in @sqlite3_step': 'bp-fj1',
                 'bennett: unimplemented: §17.4 CC marker/call lowering (milestone 1) at tail call void @__bennett_qprep': 'bp-vsa',
                 'probe/VM-state-out: exit 1': 'bennett-vm-1wf'}
        for error, bead in cases.items():
            self.assertEqual([r['id'] for r in rows if re.search(r['pattern'], error)], [bead])


class Propagation(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.g = gate.Gate(args(self.d.name))

    def tearDown(self):
        self.d.cleanup()

    def test_known_failure_stays_red_and_blocks_consumers(self):
        self.g.known = [{'id': 'test-bead', 'pattern': 'boom'}]
        def fail():
            raise gate.GateError('boom')
        self.g.stage(18, 'source', fail)
        self.g.stage(18, 'consumer', lambda: self.fail('ran blocked consumer'), ('source',))
        self.assertEqual(self.g.rows[18]['source']['status'], 'FAIL')
        self.assertEqual(self.g.rows[18]['consumer']['status'], 'BLOCKED')
        self.assertEqual(self.g.rows[18]['consumer']['beads'], ['test-bead'])

    def test_every_stage_counts_toward_exit_including_diagnostics(self):
        for m in (18, 23):
            for s in gate.STAGES:
                self.g.rows[m][s] = {'status': 'PASS', 'beads': [], 'error': ''}
        self.assertEqual(self.g.finish(), 0)
        self.g.rows[23][gate.STAGES[-1]] = {'status': 'FAIL', 'beads': ['x'], 'error': 'e'}
        self.assertEqual(self.g.finish(), 1)

    def test_acceptance_stages_exist_for_each_clause_and_executor(self):
        for ex in ('reference', 'VM'):
            for c in ('(a) oracle = native, fault rate zero', '(a) full tuple on D', '(a) literal step check',
                      '(b) amplitudes', '(b) outcomes'):
                self.assertIn(f'{ex} {c}', gate.STAGES)
        self.assertIn('(c) resource estimate at N=2^20', gate.STAGES)
        self.assertIn('(a) certificate verdict', gate.STAGES)


if __name__ == '__main__':
    unittest.main()
