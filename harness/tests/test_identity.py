"""§9.2 identity recomputed by the harness, judged against published vectors and stored hashes."""
import glob
import json
import os
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "lib"))
import objfile  # noqa: E402
import identity  # noqa: E402

WORKSPACE = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
INC8 = """bennett-object isa=0.8 object=0.8 launch=0.8 sched=0.8 cert=0.8 coverage=0.8 target=0.8 ptr_width=32

section symbols
{"0":{"interface_hash":"8ce5e64f15830d8e596ba9b4861def6565dd567b5804300f427d8b9566d5be93","kind":"oracle","name":"inc8","signature":{"abi":"0.8","args":[],"fault":4,"in":[8],"kind":"oracle","mem":[],"tgt":8,"tier":"q"}}}

section functions
oracle @inc8 (x:wire[8] in) (t:wire[8]) (f:wire[4]) () ()
regs { one:reg[8], r:reg[8] }
^entry():
one = lift.const.i8 1
r = lift.add x one
lift.retval r
"""


class ObjFile(unittest.TestCase):
    def test_round_trip_is_byte_exact(self):
        self.assertEqual(objfile.parse(INC8).text(), INC8)

    def test_functions_are_split_with_kind_and_name(self):
        o = objfile.parse(INC8)
        self.assertEqual([(f.kind, f.name) for f in o.functions], [("oracle", "inc8")])
        self.assertEqual(len(o.functions[0].lines), 6)

    def test_unknown_section_is_loud(self):
        with self.assertRaises(objfile.ObjectError):
            objfile.parse(INC8.replace("section functions", "section nonsense"))


class Identity(unittest.TestCase):
    def test_spec_example_1(self):
        self.assertEqual(identity.hashes(objfile.parse(INC8))["inc8"]["interface"],
                         "8ce5e64f15830d8e596ba9b4861def6565dd567b5804300f427d8b9566d5be93")

    def test_rehash_replaces_a_corrupted_stored_hash(self):
        bad = INC8.replace("8ce5e64f", "00000000")
        o = objfile.parse(bad)
        self.assertNotEqual(identity.stored_mismatches(o), [])
        identity.rehash(o)
        self.assertEqual(o.text(), INC8)

    def test_every_published_valid_object_recomputes_its_stored_hashes(self):
        roots = [
            os.path.join(WORKSPACE, "bennett-isa-spec/reference-interpreter/tests/fixtures/*.bisa"),
            os.path.join(WORKSPACE, "bennett-isa-spec/conformance/cases/objects/[!f]*.bisa"),
            os.path.join(WORKSPACE, "bennett-conformance/suite/cases/*/object.bisa"),
            os.path.join(WORKSPACE, "bennett-isa-spec/reference-interpreter/tests/fixtures/**/*.bisa"),
            os.path.join(WORKSPACE, "bennett-conformance/ladder/**/*.bennett"),
            os.path.join(WORKSPACE, "bennett-conformance/ladder/**/*.bisa"),
        ]
        paths = sorted({p for r in roots for p in glob.glob(r, recursive=True)})
        self.assertGreater(len(paths), 80)
        bad = {}
        for p in paths:
            with open(p, encoding="utf-8", newline="") as f:
                text = f.read()
            try:
                o = objfile.parse(text)
            except objfile.ObjectError as e:
                bad[p] = str(e)
                continue
            m = identity.stored_mismatches(o)
            if m:
                bad[p] = m
        self.assertEqual({k: v for k, v in bad.items() if "corrupt" not in k}, {})

    def test_suite_closure_hashes_match_expectations(self):
        n = 0
        for p in sorted(glob.glob(os.path.join(WORKSPACE, "bennett-conformance/suite/cases/*/expect.json"))):
            with open(p, encoding="utf-8") as f:
                e = json.load(f)
            if "hashes" not in e:
                continue
            with open(os.path.join(os.path.dirname(p), "object.bisa"), encoding="utf-8", newline="") as f:
                o = objfile.parse(f.read())
            got = identity.hashes(o)
            for name, want in e["hashes"]["symbols"].items():
                self.assertEqual(got[name], want, "%s %s" % (p, name))
                n += 1
        self.assertGreater(n, 50)


if __name__ == "__main__":
    unittest.main()
