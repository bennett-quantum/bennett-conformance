"""Classification and synthetic-corpus honesty; the actual differential runs from the CLI."""
import unittest
from checker_replay.differential import compare_claim, object_mutants, rebound_pair
from checker_replay.objparse import parse_object
from checker_replay.replay import check_certificate
from checker_replay.synthetic import synthetic_cases
from tests.spec_vectors import arith_objects


class Classify(unittest.TestCase):
    def test_outcome_and_fraction(self):
        self.assertIsNone(compare_claim({"outcome": "reject"}, {"outcome": "reject"}))
        self.assertEqual(compare_claim({"outcome": "reject"}, {"outcome": "proof"}), "reject|proof")
        self.assertEqual(compare_claim({"outcome": "proof"}, None), "proof|absent")
        same = {"outcome": "proof", "elements": 4, "covered_fraction": "1"}
        self.assertIsNone(compare_claim(same, {"outcome": "proof", "elements": "4", "covered_fraction": "1/1"}))
        self.assertEqual(compare_claim(same, {"outcome": "proof", "elements": "4", "covered_fraction": "1/2"}), "proof|proof|fraction")
        self.assertEqual(compare_claim(same, {"outcome": "proof", "elements": "8", "covered_fraction": "1"}), "proof|proof|elements")


class Synthetic(unittest.TestCase):
    def test_every_hand_relation_replays(self):
        """Each `fact` is hand-derived from the spec: a reject here is a replay or a fact bug."""
        cases = synthetic_cases()
        self.assertGreater(len(cases), 50)
        for name, text, cert in cases:
            with self.subTest(case=name):
                v = check_certificate(cert, parse_object(text), text)
                self.assertEqual(v["errors"], [])
                claim = next(iter(v["regions"].values()))["claims"][0]
                self.assertIn(claim["outcome"], ("proof", "evidence"), claim)


class ObjectMutants(unittest.TestCase):
    def test_rebinding_keeps_only_semantics_as_objection(self):
        p = [p for p in arith_objects() if p.name == "arith.add.cuccaro.w1.bisa"][0]
        text = p.read_bytes().decode()
        muts = object_mutants(text)
        self.assertTrue(muts)
        for tag, edited in muts:
            new_text, cert = rebound_pair(text, edited)
            v = check_certificate(cert, parse_object(new_text), new_text)
            self.assertEqual(v["errors"], [], tag)


class SameBytes(unittest.TestCase):
    def test_replay_sees_the_canonical_bytes_the_binary_sees(self):
        """A float 1.0 mutant canonicalises to 1: the replay must judge the bytes, not the dict."""
        import json, pathlib, tempfile
        from checker_replay import differential
        seen = {}
        orig = differential.check_certificate

        def spy(cert, *a, **k):
            seen["cert"] = cert
            return {"overall": "replayed", "regions": {}, "errors": []}
        differential.check_certificate = spy
        differential_run = differential.run_binary
        differential.run_binary = lambda *a, **k: ({"overall": "verified", "regions": {}, "errors": []}, 0, "")
        try:
            with tempfile.TemporaryDirectory(dir=pathlib.Path(__file__).resolve().parents[2] / ".tmp") as d:
                run = differential.Run("/bin/true", d)
                p = [p for p in arith_objects() if p.name == "arith.add.cuccaro.w1.bisa"][0]
                text = p.read_bytes().decode()
                lines = text.split('\n')
                cert = json.loads(lines[lines.index('section certificate') + 1])
                key = next(iter(cert["regions"]))
                cert["regions"][key]["claims"][0]["domain"]["bundles"][0]["hi"] = 1.0
                run.compare("t", "t", text, cert, "s")
        finally:
            differential.check_certificate = orig
            differential.run_binary = differential_run
        hi = seen["cert"]["regions"][key]["claims"][0]["domain"]["bundles"][0]["hi"]
        self.assertIs(type(hi), int)
