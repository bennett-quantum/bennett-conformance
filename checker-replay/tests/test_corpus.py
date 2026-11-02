"""Every published 0.9 certificate replays: arith objects and the controlled fixture."""
import json, unittest
from checker_replay.objparse import parse_object
from checker_replay.replay import check_certificate
from tests.spec_vectors import arith_objects, embedded_certificate


class Arith(unittest.TestCase):
    def test_every_exhaustive_claim_is_a_proof(self):
        totals = {"proof": 0, "evidence": 0, "reject": 0, "unchecked": 0}
        for p in arith_objects():
            text = p.read_bytes().decode()
            obj = parse_object(text)
            v = check_certificate(embedded_certificate(text), obj, text)
            self.assertEqual(v["errors"], [], p.name)
            for key, rec in v["regions"].items():
                for c in rec["claims"]:
                    totals[c["outcome"]] += 1
                    if c["kind"] == "exhaustive":
                        self.assertEqual(c["outcome"], "proof", (p.name, key, c))
                    else:
                        self.assertEqual(c["outcome"], "unchecked", (p.name, c))
        self.assertGreater(totals["proof"], 40, totals)
