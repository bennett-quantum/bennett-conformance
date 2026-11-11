"""A diagnostic selection must never enter the full SQLite pipeline."""
import argparse
from contextlib import redirect_stdout
import io
import json
import os
import tempfile
import unittest

import gate


class SelectionTests(unittest.TestCase):
    def run_gate(self, diagnostics_only):
        directory = tempfile.TemporaryDirectory(dir=os.environ['TMPDIR'])
        self.addCleanup(directory.cleanup)
        runner = gate.Gate(argparse.Namespace(output=directory.name, diagnostics_only=diagnostics_only))
        calls = []
        runner.sources.prepare = lambda: calls.append('prepare')
        for method in ('reference', 'fixtures', 'sqlite_source', 'predicate', 'plugin', 'vm', 'driver',
                       'estimator', 'checker', 'host', 'frontend', 'runtime_link', 'postlink', 'closure',
                       'production', 'artefacts', 'comparison', 'admission', 'oracle_runs', 'tuples',
                       'step_checks', 'amplitudes', 'outcomes', 'certificate', 'estimate', 'control',
                       'mem_file', 'state_out', 'grover_tu', 'estimator_cli', 'checker_cli', 'images_agree'):
            setattr(runner, method, lambda *a, method=method: calls.append(method))
        with redirect_stdout(io.StringIO()):
            result = runner.main()
        return runner, calls, result

    def test_diagnostic_selection_does_not_execute_sqlite_or_claim_acceptance(self):
        runner, calls, result = self.run_gate(True)
        for method in ('host', 'frontend', 'runtime_link', 'postlink', 'closure', 'production',
                       'artefacts', 'comparison', 'admission', 'oracle_runs', 'tuples', 'step_checks',
                       'amplitudes', 'outcomes', 'certificate', 'estimate', 'images_agree'):
            self.assertNotIn(method, calls, method)
        for method in ('grover_tu', 'estimator_cli', 'checker_cli'):
            self.assertEqual(calls.count(method), 2)
        for method in ('control', 'mem_file', 'state_out'):
            self.assertEqual(calls.count(method), 4)
        self.assertEqual(result, 1)
        for major in (18, 23):
            self.assertEqual(set(runner.rows[major]), set(gate.STAGES))
            for stage in ('coherent production', 'reference (a) full tuple on D',
                          'VM (b) amplitudes', '(a) certificate verdict', '(c) resource estimate at N=2^20'):
                self.assertEqual(runner.rows[major][stage]['status'], 'NOT RUN')
        report = json.loads((runner.run/'report.json').read_text())
        self.assertEqual(report['manifest']['selection']['mode'], 'diagnostics-only')
        self.assertIn('coherent production', report['manifest']['selection']['excluded_rows'])

    def test_default_mode_still_runs_the_complete_required_pipeline(self):
        runner, calls, result = self.run_gate(False)
        self.assertEqual(result, 0)
        self.assertEqual(calls.count('production'), 2)
        self.assertEqual(calls.count('oracle_runs'), 4)
        self.assertEqual(calls.count('estimate'), 2)
        self.assertTrue(all(row['status'] == 'PASS' for rows in runner.rows.values() for row in rows.values()))


if __name__ == '__main__':
    unittest.main()
