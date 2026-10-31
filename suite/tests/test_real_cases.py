"""The imported cases against a real implementation.

Needs BENNETT_ISA_BINARY (e.g. the reference interpreter's bennett-isa-ref).
Without it this test FAILS rather than skips: a silently skipped conformance
run is a failure (see FORMAT.md).
"""
import os
import subprocess
import sys
import unittest

SUITE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


class RealCases(unittest.TestCase):
    def test_every_imported_case_passes_against_the_binary(self):
        binary = os.environ.get("BENNETT_ISA_BINARY")
        self.assertTrue(binary, "set BENNETT_ISA_BINARY to the implementation under test")
        cases = os.path.join(SUITE, "cases")
        n = len([d for d in os.listdir(cases) if os.path.isdir(os.path.join(cases, d))])
        self.assertGreaterEqual(n, 16, "the 16 seed fixtures must all be imported")
        p = subprocess.run([sys.executable, "-I", os.path.join(SUITE, "run.py"),
                            "--binary", binary, "--cases", cases],
                           capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
        self.assertIn("checked %d/%d cases" % (n, n), p.stdout)
        # no case may pass on hash/check alone while claiming runs it never made
        for line in p.stdout.splitlines()[1:n + 1]:
            self.assertTrue(line.rstrip().endswith("PASS"), line)


if __name__ == "__main__":
    unittest.main()
