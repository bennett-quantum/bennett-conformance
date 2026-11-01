"""Generators: seeded determinism, well-formedness against a real executor, shrink usability."""
import os
import shutil
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "lib"))
import executors  # noqa: E402
import generate  # noqa: E402
import identity  # noqa: E402
import objfile  # noqa: E402
import prng  # noqa: E402

REPO = os.path.abspath(os.path.join(HERE, "..", ".."))


def reference():
    path = os.environ.get("BENNETT_HARNESS_REF", os.path.join(REPO, ".tmp", "opus-harness", "bin", "bennett-isa-ref"))
    if not os.access(path, os.X_OK):
        raise AssertionError("real reference binary required (BENNETT_HARNESS_REF): %s" % path)
    return executors.Executor("ref", [path], timeout=30.0)


class Prng(unittest.TestCase):
    def test_splitmix_reference_vector(self):
        r = prng.Prng(0)
        self.assertEqual(r.u64(), 0xE220A8397B1DCDAF)
        self.assertEqual(r.u64(), 0x6E789E6AA1B965F4)

    def test_below_is_in_range(self):
        r = prng.Prng(5)
        self.assertTrue(all(0 <= r.below(7) < 7 for _ in range(1000)))


class Generated(unittest.TestCase):
    def test_same_seed_same_bytes_and_argv(self):
        for cls in generate.CLASSES:
            a, b = generate.program(cls, 42), generate.program(cls, 42)
            self.assertEqual(a.text, b.text, cls)
            self.assertEqual(a.invocations, b.invocations, cls)

    def test_seeds_vary_programs(self):
        for cls in generate.CLASSES:
            self.assertGreater(len({generate.program(cls, s).text for s in range(20)}), 15, cls)

    def test_stored_hashes_match_section_9_2(self):
        for cls in generate.WELL_FORMED:
            for s in range(40):
                o = objfile.parse(generate.program(cls, s).text)
                self.assertEqual(identity.stored_mismatches(o), [], "%s seed %d" % (cls, s))

    def test_programs_are_accepted_by_the_reference(self):
        ref = reference()
        tmp = tempfile.mkdtemp(dir=os.environ.get("TMPDIR"))
        self.addCleanup(shutil.rmtree, tmp)
        for cls in generate.WELL_FORMED:
            for s in range(60):
                p = generate.program(cls, s)
                path = os.path.join(tmp, "o.bisa")
                with open(path, "w", encoding="utf-8", newline="") as f:
                    f.write(p.text)
                o = ref.run(["check", path])
                self.assertEqual(o.kind, "json", "%s seed %d: %s\n%s" % (cls, s, o.stderr, p.text))

    def test_q_class_covers_structured_opcodes(self):
        text = "\n".join(generate.program("q", s).text for s in range(120))
        for token in ("qif ", "qloop ", " while ", "qcopy ", "qregion ", "ctl ", "qalloc ", "rz ", "h "):
            self.assertIn(token, text)

    def test_r_class_covers_opcodes(self):
        text = "\n".join(generate.program("r", s).text for s in range(120))
        for token in ("effect clock.read", "effect entropy.read", "effect console.write", "effect block.write",
                      "uncall @", "load.i", "store.i", "sdiv", "inttoptr", "br "):
            self.assertIn(token, text)

    def test_lift_class_covers_rows(self):
        text = "\n".join(generate.program("lift", s).text for s in range(120))
        for token in ("lift.call @", "lift.ret ", "qbranch ", "qjumpr ", "lift.label", "lift.load", "lift.store",
                      "lift.alloc", "lift.free", "lift.faultif", "lift.parmov", "fault "):
            self.assertIn(token, text)

    def test_mutants_keep_section_9_2_identity_when_they_parse(self):
        for s in range(40):
            o = objfile.parse(generate.program("mutant", s).text)
            self.assertEqual(identity.stored_mismatches(o), [], "seed %d" % s)

    def test_cc_class_covers_quantum_control(self):
        text = "\n".join(generate.program("cc", s).text for s in range(120))
        for token in ("measure ", "qreg.new", "qreg.slice", "qreg.concat", "action_call @", "oracle_call @", "fn cc @"):
            self.assertIn(token, text)
        self.assertTrue(any("--draws" in inv[0] for s in range(5) for inv in generate.program("cc", s).invocations))


if __name__ == "__main__":
    unittest.main()
