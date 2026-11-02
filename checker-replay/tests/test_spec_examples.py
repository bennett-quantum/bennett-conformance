"""§16.3 normative vectors: parser, §9.2 hashes, and the stated reference semantics."""
import unittest
from checker_replay.objparse import parse_object
from checker_replay import hashes
from checker_replay.oracle import oracle_payload, oracle_map
from checker_replay.action import compile_action
from tests.spec_vectors import example_objects, arith_objects

EX = example_objects()
DB = list(range(1, 17))


def sum_db(n, budget, cells=DB):
    return oracle_payload(parse_object(EX[2]), "sum_db", [n], [list(cells)], [], budget)


class HashVectors(unittest.TestCase):
    def test_every_example_symbol_interface_hash(self):
        for k, text in EX.items():
            obj = parse_object(text)
            h = hashes.compute_hashes(obj)
            for s in obj.symbols:
                with self.subTest(example=k, symbol=s["name"]):
                    self.assertEqual(h[s["name"]][0], s["interface_hash"])

    def test_every_published_arith_object_hashes(self):
        for p in arith_objects():
            text = p.read_text()
            obj = parse_object(text)
            h = hashes.compute_hashes(obj)
            for s in obj.symbols:
                self.assertEqual(h[s["name"]][0], s["interface_hash"], (p.name, s["name"]))

    def test_example2_printed_hashes(self):
        h = hashes.compute_hashes(parse_object(EX[2]))
        self.assertEqual(h["zero"][0], "43e032f98d7aff20f6864d6368b20ab4ef9d810938999acff8d71ed16e6b17d7")
        self.assertEqual(h["sum"][0], "cb52102decd586b7585b5d73b90841dcec75edac5a5a2d0f43c5efb34d4cceb1")
        self.assertEqual(h["sum_db"][0], "3ad937b1cd867454989c5c5f58244681ef2c09dbabf7fce56515a68f63462ee6")


class Example2Semantics(unittest.TestCase):
    BIG = (100000, 0, 512, 100)

    def test_sum_for_every_n_up_to_16(self):
        for n in range(17):
            self.assertEqual(sum_db(n, self.BIG), (sum(DB[:n]) % 256, 0), n)

    def test_n_above_16_memfaults(self):
        for n in (17, 18, 40, 255):
            self.assertEqual(sum_db(n, self.BIG), (0, 7), n)

    def test_small_B_is_budget_steps(self):
        self.assertEqual(sum_db(3, (5, 0, 512, 100)), (0, 1))
        self.assertEqual(sum_db(0, (0, 0, 512, 100)), (0, 1))

    def test_depth_limit_le_n_is_budget_depth(self):
        # "with depth_limit <= n ... FAULT_BUDGET_DEPTH"
        for n in (1, 2, 5):
            self.assertEqual(sum_db(n, (100000, 0, 512, n)), (0, 3), n)
            self.assertEqual(sum_db(n, (100000, 0, 512, n + 1)), (sum(DB[:n]) % 256, 0), n)

    def test_monotonic_stack_16_cells_per_call(self):
        # "with stack_limit < 16(n+1) ... FAULT_BUDGET_DEPTH"; n=16 needs 272
        self.assertEqual(sum_db(16, (100000, 0, 271, 100)), (0, 3))
        self.assertEqual(sum_db(16, (100000, 0, 272, 100)), (sum(DB) % 256, 0))
        self.assertEqual(sum_db(2, (100000, 0, 47, 100)), (0, 3))
        self.assertEqual(sum_db(2, (100000, 0, 48, 100)), (sum(DB[:2]), 0))


class Example1Semantics(unittest.TestCase):
    def test_inc8_all_x_and_budget(self):
        obj = parse_object(EX[1])
        for x in range(256):
            self.assertEqual(oracle_payload(obj, "inc8", [x], [], [], (3, 0, 0, 0)), ((x + 1) % 256, 0))
            self.assertEqual(oracle_payload(obj, "inc8", [x], [], [], (2, 0, 0, 0)), (0, 1))

    def test_involution_on_target_and_fault(self):
        obj = parse_object(EX[1])
        self.assertEqual(oracle_map(obj, "inc8", [5], 0b1010, 3, [], [], (3, 0, 0, 0)), (0b1010 ^ 6, 3))
        self.assertEqual(oracle_map(obj, "inc8", [5], 0b1010, 3, [], [], (2, 0, 0, 0)), (0b1010, 3 ^ 1))


class Example3Semantics(unittest.TestCase):
    def test_borrow_demo_8_and_controlled_16(self):
        obj = parse_object(EX[3])
        p0 = compile_action(obj, "borrow_demo", [], 0)
        self.assertEqual(p0.borrow_widths, [1])
        p1 = compile_action(obj, "borrow_demo", [], 1)
        for t in (0, 1):
            for x in (0, 1):
                for b in (0, 1):
                    self.assertEqual(p0.run([t, x], [b], []), ([t ^ x, x], [b], []))
                    for w in (0, 1):
                        self.assertEqual(p1.run([w, t, x], [b], []), ([w, t ^ (w & x), x], [b], []))
