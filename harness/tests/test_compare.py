"""Comparator: every field has a labelled mode; nothing unknown is silently ignored."""
import os
import sys
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "lib"))
import compare  # noqa: E402


class Compare(unittest.TestCase):
    def diff(self, a, b):
        return compare.compare_json(a, b)

    def test_identical_is_exact(self):
        d = self.diff({"a": 1, "s": {"0": [0.5, 0.0]}}, {"a": 1, "s": {"0": [0.5, 0.0]}})
        self.assertEqual(d.hard, [])
        self.assertEqual(d.tolerated, [])

    def test_unknown_field_only_on_one_side_is_a_divergence(self):
        self.assertEqual([x.path for x in self.diff({"a": 1}, {"a": 1, "b": 2}).hard], ["b"])

    def test_true_is_not_one(self):
        self.assertEqual(len(self.diff({"a": True}, {"a": 1}).hard), 1)

    def test_integer_values_differ_hard(self):
        self.assertEqual([x.path for x in self.diff({"steps": 3}, {"steps": 4}).hard], ["steps"])

    def test_list_length_and_elements_are_located(self):
        d = self.diff({"tape": [{"op": "a"}, {"op": "b"}]}, {"tape": [{"op": "a"}, {"op": "c"}]})
        self.assertEqual([x.path for x in d.hard], ["tape[1].op"])

    def test_amplitude_within_tolerance_is_tolerated_and_labelled(self):
        d = self.diff({"state": {"01": [0.5, 0.0]}}, {"state": {"01": [0.5 + 1e-15, 0.0]}})
        self.assertEqual(d.hard, [])
        self.assertEqual([x.path for x in d.tolerated], ["state.01[0]"])
        self.assertEqual(d.tolerated[0].mode, "tolerance")

    def test_amplitude_beyond_tolerance_is_hard(self):
        self.assertEqual(len(self.diff({"state": {"01": [0.5, 0.0]}}, {"state": {"01": [0.6, 0.0]}}).hard), 1)

    def test_negative_zero_is_not_exact(self):
        d = self.diff({"state": {"0": [0.0, -0.0]}}, {"state": {"0": [0.0, 0.0]}})
        self.assertEqual(d.hard, [])
        self.assertEqual(len(d.tolerated), 1)

    def test_vanishing_component_is_tolerated_only_when_tiny(self):
        d = self.diff({"state": {"0": [1.0, 0.0], "1": [1e-16, 0.0]}}, {"state": {"0": [1.0, 0.0]}})
        self.assertEqual(d.hard, [])
        self.assertEqual(len(d.tolerated), 1)
        d = self.diff({"state": {"0": [1.0, 0.0], "1": [0.5, 0.0]}}, {"state": {"0": [1.0, 0.0]}})
        self.assertEqual(len(d.hard), 1)

    def test_float_outside_state_is_exact(self):
        self.assertEqual(len(self.diff({"results": [{"value": 0.5}]}, {"results": [{"value": 0.5 + 1e-15}]}).hard), 1)

    def test_opaque_digests_are_compared_relationally(self):
        same = {"memory_digest": "aa", "entry_memory_digest": "aa"}
        d = self.diff(same, {"memory_digest": "bb", "entry_memory_digest": "bb"})
        self.assertEqual(d.hard, [])
        self.assertEqual(d.relational, ["memory_digest"])
        d = self.diff(same, {"memory_digest": "bb", "entry_memory_digest": "cc"})
        self.assertEqual([x.path for x in d.hard], ["memory_digest==entry_memory_digest"])

    def test_first_difference_compares_nullness(self):
        self.assertEqual(self.diff({"first_difference": "x"}, {"first_difference": "y"}).hard, [])
        self.assertEqual(len(self.diff({"first_difference": "x"}, {"first_difference": None}).hard), 1)


class Classify(unittest.TestCase):
    def test_kinds(self):
        k = compare.classify
        self.assertEqual(k(0, b'{"a":1}', "", False), "json")
        self.assertEqual(k(1, b"", "E_ARG", False), "diagnostic")
        self.assertEqual(k(1, b"", "unsupported: milestone 9", False), "unsupported")
        self.assertEqual(k(1, b"", "x: staged deliverable 9", False), "unsupported")
        self.assertEqual(k(1, b"", "§1: unsupported ISA version 0.7", False), "diagnostic")
        self.assertEqual(k(1, b"", "§5.2: unresolved; needs linking (milestone 2)", False), "diagnostic")
        self.assertEqual(k(1, b"", "§11: E_X: unimplemented: library (milestone 4)", False), "unsupported")
        self.assertEqual(k(101, b"", "panicked", False), "crash")
        self.assertEqual(k(-11, b"", "", False), "crash")
        self.assertEqual(k(0, b"nope", "", False), "badoutput")
        self.assertEqual(k(1, b"{}", "E", False), "badoutput")
        self.assertEqual(k(None, b"", "", True), "timeout")


class Oracle(unittest.TestCase):
    def test_subset_ignores_extra_object_fields(self):
        self.assertEqual(compare.subset_problems({"a": 1}, {"a": 1, "b": 2}), [])

    def test_subset_reports_missing_and_wrong(self):
        self.assertEqual(len(compare.subset_problems({"a": 1}, {"b": 2})), 1)
        self.assertEqual(len(compare.subset_problems({"a": 1}, {"a": 2})), 1)

    def test_subset_number_as_binary64(self):
        self.assertEqual(compare.subset_problems({"a": 1}, {"a": 1.0}), [])

    def test_subset_booleans(self):
        self.assertEqual(compare.subset_problems({"a": True}, {"a": True}), [])
        self.assertEqual(len(compare.subset_problems({"a": True}, {"a": False})), 1)
        self.assertEqual(len(compare.subset_problems({"a": True}, {"a": 1})), 1)

    def test_exact_expectation_treats_digests_as_opaque(self):
        e = {"exit": 0, "exact_json": {"a": 1, "memory_digest": "aa"}}
        self.assertEqual(compare.judge(e, 0, b'{"a":1,"memory_digest":"bb"}', ""), [])
        self.assertEqual(len(compare.judge(e, 0, b'{"a":2,"memory_digest":"bb"}', "")), 1)

    def test_subset_arrays_have_exact_length(self):
        self.assertEqual(len(compare.subset_problems({"a": [1]}, {"a": [1, 2]})), 1)


if __name__ == "__main__":
    unittest.main()
