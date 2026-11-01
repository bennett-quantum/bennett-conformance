"""Corpora accounting: every published vector, symbol and invocation is run or a counted skip."""
import glob
import json
import os
import sys
import tempfile
import shutil
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "lib"))
import corpora  # noqa: E402
import drivers  # noqa: E402
import engine  # noqa: E402
import executors  # noqa: E402
import objfile  # noqa: E402

REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
WORKSPACE = os.path.dirname(REPO)
FAKE = os.path.join(HERE, "fake_exec.py")


class Accounting(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(dir=os.environ.get("TMPDIR"))
        self.addCleanup(shutil.rmtree, self.tmp)
        e = [executors.Executor(n, [sys.executable, "-I", FAKE]) for n in ("a", "b")]
        self.r = engine.Runner(e, workdir=self.tmp)

    def test_every_isa_vector_file_is_a_case_or_a_named_skip(self):
        files = glob.glob(os.path.join(WORKSPACE, "bennett-isa-spec/conformance/cases/*/*/case.json"))
        extra = glob.glob(os.path.join(WORKSPACE, "bennett-isa-spec/conformance/fixtures/*/*"))
        cases = list(corpora.isa_vectors(WORKSPACE, self.r))
        skipped = sum(self.r.corpora["isa-vectors"]["skipped"].values())
        self.assertEqual(len(cases) + skipped, len(files) + len(extra))
        self.assertGreater(len(cases), 400)

    def test_vector_expectations_carry_through(self):
        cases = {c.id: c for c in corpora.isa_vectors(WORKSPACE, self.r)}
        c = cases["B-boundary-halt-exact"]
        self.assertEqual(c.argv[1], "$OBJECT")
        self.assertEqual(c.expect["stdout_json"], {"code": 0, "payload": 9, "steps": 3})

    def test_neutral_suite_has_one_case_per_run_record(self):
        want = 0
        for p in glob.glob(os.path.join(REPO, "suite/cases/*/expect.json")):
            with open(p, encoding="utf-8") as f:
                e = json.load(f)
            want += len(e.get("runs", ())) + ("hashes" in e) + ("check" in e)
        self.assertEqual(len(list(corpora.neutral_suite(REPO, self.r))), want)

    def test_vm_differential_is_data_and_accounts_every_list(self):
        with open(os.path.join(REPO, "harness/corpora/vm-differential.json"), encoding="utf-8") as f:
            d = json.load(f)
        self.assertRegex(d["source"], r"bennett-vm tests/differential.rs @ [0-9a-f]{40}")
        cases = list(corpora.vm_differential(WORKSPACE, self.r))
        self.assertEqual(len(cases), len(d["invocations"]))
        self.assertEqual(sum(self.r.corpora["vm-differential"]["skipped"].values()), len(d["skipped"]))

    def test_driver_accounts_for_every_symbol(self):
        path = os.path.join(WORKSPACE, "bennett-isa-spec/reference-interpreter/tests/fixtures/sum_db.bisa")
        with open(path, encoding="utf-8", newline="") as f:
            o = objfile.parse(f.read())
        cases = list(drivers.for_object("c", "x", self.r, object_path=path, o=o))
        driven = {c.id.split("/")[1] for c in cases} - {"check", "hash"}
        skipped = sum(self.r.corpora["c"]["skipped"].values())
        self.assertEqual(len(driven) + skipped, len(o.symbol_table()))

    def test_driver_spellings_cover_every_classical_type(self):
        for t in ("i1", "i128", "ptr", "f64", "token", "qreg[3]"):
            self.assertTrue(drivers._type_value(t, 0))
        with self.assertRaises(ValueError):
            drivers._type_value("region[1,8]", 0)


if __name__ == "__main__":
    unittest.main()
