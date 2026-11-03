"""Engine: classification is total, divergences carry minimised witnesses, nothing is silently skipped."""
import json
import os
import shutil
import stat
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "lib"))
import engine  # noqa: E402
import executors  # noqa: E402
import identity  # noqa: E402
import objfile  # noqa: E402
import shrink  # noqa: E402

FAKE = os.path.join(HERE, "fake_exec.py")
HEAD = "bennett-object isa=0.8 object=0.8 launch=0.8 sched=0.8 cert=0.8 coverage=0.8 target=0.8 ptr_width=16\n"


def program():
    body = ["fn r @main (a:i8, b:i8) (r:i8) () () ()", "^entry():"]
    for i in range(30):
        body.append("c%d = add a b" % i)
    body += ["k = xor a b"]
    for i in range(30, 60):
        body.append("c%d = sub a b" % i)
    body += ["r = add a b", "ret r"]
    sig = {"abi": "0.8", "kind": "r", "params": ["i8", "i8"], "results": ["i8"], "tier": "r"}
    helper = ["fn r @helper () () () () ()", "^entry():", "ret"]
    sym = {"0": {"interface_hash": "", "kind": "r", "name": "main", "signature": sig},
           "1": {"interface_hash": "", "kind": "r", "name": "helper",
                 "signature": {"abi": "0.8", "kind": "r", "params": [], "results": [], "tier": "r"}}}
    text = HEAD + "\nsection symbols\n" + objfile.jcs(sym) + "\n\nsection functions\n" + "\n".join(body + helper) + "\n"
    return identity.rehash(objfile.parse(text)).text()


def fake(name, mode, **env):
    return executors.Executor(name, [sys.executable, "-I", FAKE], dict(env, FAKE_MODE=mode), timeout=2.0)


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(dir=os.environ.get("TMPDIR"))
        self.addCleanup(shutil.rmtree, self.tmp)
        self.text = program()

    def case(self, argv=None, **kw):
        return engine.Case("t", "c1", argv or ["run-cc", "$OBJECT", "main", "--arg", "7", "--arg", "9"],
                           object_text=self.text, **kw)

    def run_one(self, a, b, case=None, **kw):
        r = engine.Runner([fake("a", a), fake("b", b)], workdir=self.tmp, witness_dir=os.path.join(self.tmp, "w"),
                          shrink_seconds=20.0, **kw)
        return r, r.run_case(case or self.case())


class Classes(Base):
    def test_identical_is_exact(self):
        _, res = self.run_one("base", "base")
        self.assertEqual(res.pairs[("a", "b")].cls, "MATCH_EXACT")

    def test_tolerated_amplitude_is_labelled_not_exact(self):
        _, res = self.run_one("base", "amp")
        self.assertEqual(res.pairs[("a", "b")].cls, "MATCH_TOLERANCE")

    def test_explicit_unsupported_is_counted_with_reason(self):
        r, res = self.run_one("base", "unsupported")
        self.assertEqual(res.pairs[("a", "b")].cls, "UNSUPPORTED")
        self.assertEqual(r.summary()["unsupported_reasons"]["b"], {"unsupported: milestone 9": 1})

    def test_crash_is_a_divergence_not_a_reject(self):
        _, res = self.run_one("base", "crash")
        self.assertEqual(res.pairs[("a", "b")].cls, "DIVERGE")

    def test_both_reject_is_its_own_class(self):
        _, res = self.run_one("reject", "reject")
        self.assertEqual(res.pairs[("a", "b")].cls, "AGREE_REJECT")

    def test_reject_versus_success_diverges(self):
        _, res = self.run_one("base", "reject")
        self.assertEqual(res.pairs[("a", "b")].cls, "DIVERGE")

    def test_timeout_is_counted(self):
        _, res = self.run_one("base", "slow")
        self.assertEqual(res.pairs[("a", "b")].cls, "TIMEOUT")

    def test_garbage_stdout_diverges(self):
        _, res = self.run_one("base", "garbage")
        self.assertEqual(res.pairs[("a", "b")].cls, "DIVERGE")

    def test_tape_bytes_are_compared(self):
        r = engine.Runner([fake("a", "base", FAKE_TAPE="[]"), fake("b", "base", FAKE_TAPE="[ ]")],
                          workdir=self.tmp, witness_dir=os.path.join(self.tmp, "w"), shrink_seconds=0.0)
        res = r.run_case(self.case(tape=True))
        p = res.pairs[("a", "b")]
        self.assertEqual(p.cls, "DIVERGE")
        self.assertIn("$tape_bytes", [x.path for x in p.diff.hard])


class Witness(Base):
    def test_divergence_is_minimised_to_the_planted_line(self):
        r, res = self.run_one("base", "buggy")
        w = res.pairs[("a", "b")].witness
        self.assertIsNotNone(w)
        with open(os.path.join(w, "object.bisa"), encoding="utf-8") as f:
            shrunk = f.read()
        self.assertIn("xor a b", shrunk)
        self.assertNotIn("c30 =", shrunk)
        self.assertNotIn("@helper", shrunk)
        self.assertLessEqual(len(objfile.parse(shrunk).function("main").lines), 5)
        with open(os.path.join(w, "witness.json"), encoding="utf-8") as f:
            meta = json.load(f)
        self.assertEqual(meta["pair"], ["a", "b"])
        self.assertIn("value", [d["path"] for d in meta["diff"]])
        self.assertLess(meta["shrunk_lines"], meta["original_lines"])
        self.assertEqual(identity.stored_mismatches(objfile.parse(shrunk)), [])

    def test_argv_is_minimised(self):
        c = self.case(["run-cc", "$OBJECT", "main", "--arg", "7", "--arg", "9", "--draws", "0.5"])
        _, res = self.run_one("base", "buggy", case=c)
        with open(os.path.join(res.pairs[("a", "b")].witness, "witness.json"), encoding="utf-8") as f:
            self.assertEqual(json.load(f)["argv"], ["run-cc", "$OBJECT", "main"])

    def test_shrink_is_deterministic(self):
        d = lambda: engine.Runner([fake("a", "base"), fake("b", "buggy")], workdir=self.tmp,
                                  witness_dir=os.path.join(self.tmp, "w"), shrink_seconds=20.0)
        w1 = d().run_case(self.case()).pairs[("a", "b")].witness
        with open(os.path.join(w1, "object.bisa"), encoding="utf-8") as f:
            first = f.read()
        shutil.rmtree(w1)
        w2 = d().run_case(self.case()).pairs[("a", "b")].witness
        with open(os.path.join(w2, "object.bisa"), encoding="utf-8") as f:
            self.assertEqual(f.read(), first)

    def test_one_signature_only_shrinks_a_bounded_number_of_times_but_all_are_counted(self):
        r = engine.Runner([fake("a", "base"), fake("b", "buggy")], workdir=self.tmp,
                          witness_dir=os.path.join(self.tmp, "w"), shrink_seconds=5.0, witnesses_per_signature=1)
        for i in range(3):
            r.run_case(engine.Case("t", "c%d" % i, ["run-cc", "$OBJECT", "main"], object_text=self.text))
        s = r.summary()
        self.assertEqual(s["corpora"]["t"]["pairs"]["a:b"]["DIVERGE"], 3)
        sig = s["divergences"]
        self.assertEqual(len(sig), 1)
        self.assertEqual(sig[0]["count"], 3)
        self.assertEqual(len(sig[0]["witnesses"]), 1)


class Accounting(Base):
    def test_skips_are_reported_with_reason_and_count(self):
        r = engine.Runner([fake("a", "base"), fake("b", "base")], workdir=self.tmp, witness_dir=self.tmp)
        r.skip("t", "s1", "no CLI probe")
        r.skip("t", "s2", "no CLI probe")
        r.run_case(self.case())
        s = r.summary()
        self.assertEqual(s["corpora"]["t"]["skipped"], {"no CLI probe": 2})
        self.assertEqual(s["corpora"]["t"]["cases"], 1)
        self.assertEqual(s["totals"]["skipped"], 2)

    def test_every_case_is_counted_exactly_once_per_pair(self):
        r = engine.Runner([fake("a", "base"), fake("b", "reject"), fake("c", "unsupported")],
                          workdir=self.tmp, witness_dir=os.path.join(self.tmp, "w"), shrink_seconds=0.0)
        r.run_case(self.case())
        s = r.summary()
        for pair, row in s["corpora"]["t"]["pairs"].items():
            self.assertEqual(sum(row.values()), 1, pair)
        self.assertEqual(set(s["corpora"]["t"]["pairs"]), {"a:b", "a:c", "b:c"})

    def test_exit_status_contract(self):
        r, _ = self.run_one("base", "base")
        self.assertEqual(r.exit_status(), 0)
        r, _ = self.run_one("base", "unsupported")
        self.assertEqual(r.exit_status(), 2)
        r, _ = self.run_one("base", "reject")
        self.assertEqual(r.exit_status(), 1)
        r, _ = self.run_one("base", "base")
        r.skip("t", "s", "x")
        self.assertEqual(r.exit_status(), 2)
        r = engine.Runner([fake("a", "base"), fake("b", "base")], workdir=self.tmp, witness_dir=self.tmp)
        r.skip("t", "s", "x")
        self.assertEqual(r.exit_status(), 1, "everything skipped executes nothing: a failure")
        r = engine.Runner([fake("a", "base"), fake("b", "base")], workdir=self.tmp, witness_dir=self.tmp)
        self.assertEqual(r.exit_status(), 1, "a run over nothing is a failure, never a pass")


class SpecOracle(Base):
    def test_shared_wrong_wide_integer_result_fails_the_oracle(self):
        c = self.case(expect=engine.subset_expect(0, {"value": 9007199254740992}))
        r = engine.Runner([fake(n, "base", FAKE_VALUE="9007199254740993") for n in ("a", "b")],
                          workdir=self.tmp, witness_dir=os.path.join(self.tmp, "w"), shrink_seconds=0.0)
        res = r.run_case(c)
        self.assertEqual(res.pairs[("a", "b")].cls, "MATCH_EXACT")
        summary = r.summary()
        self.assertEqual(summary["corpora"]["t"]["oracle"], {"a": {"fail": 1}, "b": {"fail": 1}})
        self.assertEqual(summary["totals"]["oracle_fail"], 2)
        for failure in summary["oracle_failures"]:
            self.assertIn("$.value: expected 9007199254740992, got 9007199254740993", failure["problem"])
        self.assertEqual(r.exit_status(), 1)

    def test_oracle_failure_is_reported_for_each_executor(self):
        c = self.case(expect=engine.subset_expect(0, {"value": 2}))
        r = engine.Runner([fake("a", "base"), fake("b", "buggy")], workdir=self.tmp,
                          witness_dir=os.path.join(self.tmp, "w"), shrink_seconds=0.0)
        r.run_case(c)
        o = r.summary()["corpora"]["t"]["oracle"]
        self.assertEqual(o["a"]["fail"], 1)
        self.assertEqual(o["b"]["pass"], 1)
        self.assertEqual(r.exit_status(), 1)

    def test_shrink_module_reports_unavailable_object_shrink_for_inconsistent_hashes(self):
        bad = self.text.replace('"interface_hash":"', '"interface_hash":"0', 1)
        self.assertIsNotNone(shrink.object_shrink_blocker(bad))
        self.assertIsNone(shrink.object_shrink_blocker(self.text))


if __name__ == "__main__":
    unittest.main()
