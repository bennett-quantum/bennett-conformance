import unittest
from checker_replay import jcs, hashes
from checker_replay.objparse import parse_object
from tests.helpers import controlled
from tests.spec_vectors import example_objects

EXAMPLE1 = example_objects()[1]

class JcsTest(unittest.TestCase):
    def test_key_sorting_and_escaping(self):
        self.assertEqual(jcs.canonical({"b": 1, "a": [True, None, "x\"y\n"]}),
                         b'{"a":[true,null,"x\\"y\\n"],"b":1}')

    def test_params_hash_claim_binding_witness(self):
        # ISA 0.9 §12.3 CLAIM-BINDING witness
        args = ["18446744073709551617", "0x8000000000000000", ["2", "0", "0", "0"]]
        self.assertEqual(hashes.params_hash(args),
                         "9f628a694fa7259bf899bfb677aabce1c084e268a738a40f7f643eb6d8c9c093")

    def test_rfc8785_number_samples(self):
        # RFC 8785 appendix B values representable without a JSON parser round trip
        for x, want in ((1e21, "1e+21"), (1e20, "100000000000000000000"), (1e-6, "0.000001"), (1e-7, "1e-7"),
                        (0.000001234, "0.000001234"), (333333333.3333333, "333333333.3333333"), (-0.0, "0"),
                        (5e-324, "5e-324"), (1.7976931348623157e308, "1.7976931348623157e+308")):
            self.assertEqual(jcs.canonical(x).decode(), want)

    def test_utf16_key_order(self):
        self.assertEqual(jcs.canonical({"\U0001f600": 1, "\ufb33": 2}).decode(), '{"\U0001f600":1,"\ufb33":2}')


class HashTest(unittest.TestCase):
    def test_example1_interface_hash(self):
        obj = parse_object(EXAMPLE1)
        h = hashes.compute_hashes(obj)
        self.assertEqual(h["inc8"][0], "ed9865300b85a519c2379de98e36e635e72b5b78d8bdfca03dccf9a09aa3df5e")
        self.assertEqual(h["inc8"][1], h["inc8"][0])

    def test_controlled_object_and_closure_hashes(self):
        text, cert, _t, _s = controlled()
        self.assertEqual(hashes.object_hash(text), cert["bindings"]["object_hash"])
        obj = parse_object(text)
        h = hashes.compute_hashes(obj)
        self.assertEqual(h["controlled"][1], cert["bindings"]["closure_hash"])
        self.assertIn(h["flip"][1], cert["regions"])
