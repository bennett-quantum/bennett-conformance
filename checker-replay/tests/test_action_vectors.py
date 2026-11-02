"""Published K78-K80 basis tuples (admitted cases only) and hand-derived action semantics."""
import json, unittest
from checker_replay.objparse import parse_object
from checker_replay.action import compile_action
from checker_replay.errors import Reject, Unchecked
from tests.spec_vectors import k78_k80_vectors, example_objects

HEAD = "bennett-object isa=0.9 object=0.9 launch=0.9 sched=0.9 cert=0.9 coverage=0.9 target=0.9 ptr_width=32\n"


def act(body, hdr="(t:wire mut) (c:wire in) () () ()", regs="", kind="action", extra="", extra_sym=()):
    syms = {"0": {"interface_hash": "0" * 64, "kind": kind, "name": "a", "signature": {}}}
    for i, n in enumerate(extra_sym):
        syms[str(i + 1)] = {"interface_hash": "0" * 64, "kind": "action", "name": n, "signature": {}}
    t = HEAD + "\nsection symbols\n" + json.dumps(syms) + f"\n\nsection functions\n{kind} @a {hdr}\n{regs}^entry():\n{body}{extra}"
    return parse_object(t)


class PublishedTuples(unittest.TestCase):
    def test_all_admitted_vectors(self):
        n = 0
        for fname, v in k78_k80_vectors():
            if not v["source_admission"] or not v["tuples"]:
                continue
            obj = parse_object(v["source"])
            name = obj.symbols[0]["name"]
            prog = compile_action(obj, name, [], 1 if "control" in v["tuple_order"] else 0)
            for row in v["tuples"]:
                self.assertEqual(row["phase"], [1, 0])
                got = prog.run_flat(row["before"], v["tuple_order"])
                with self.subTest(file=fname, id=v.get("id"), before=row["before"]):
                    self.assertEqual(got, row["after"])
                n += 1
        self.assertGreater(n, 100)


class Semantics(unittest.TestCase):
    def test_qif_both_arms_and_selector_restored(self):
        # §4.6: then-arm under w, else-arm under not w, w restored
        o = act("qif c {\nx t\n} {\nh t\nh t\n}\n")
        p = compile_action(o, "a", [], 0)
        self.assertEqual(p.run([0, 1], [], []), ([1, 1], [], []))
        self.assertEqual(p.run([0, 0], [], []), ([0, 0], [], []))

    def test_second_block_parses_after_qif(self):
        o = act("qif c {\nx t\n} {\n}\n^b():\nx t\n")
        p = compile_action(o, "a", [], 0)
        self.assertEqual(p.run([0, 1], [], []), ([0, 1], [], []))
        self.assertEqual(p.run([0, 0], [], []), ([1, 0], [], []))

    def test_interference_is_exact_and_phase_is_checked(self):
        p = compile_action(act("h t\ns t\ns t\nh t\n"), "a", [], 0)  # HZH = X
        self.assertEqual(p.run([0, 0], [], []), ([1, 0], [], []))
        q = compile_action(act("t t\ntdg t\n"), "a", [], 0)
        self.assertEqual(q.run([1, 0], [], []), ([1, 0], [], []))
        z = compile_action(act("s t\ns t\n"), "a", [], 0)  # Z: -1 phase on |1>, not a +1 permutation
        self.assertEqual(z.run([0, 0], [], []), ([0, 0], [], []))
        with self.assertRaises(Reject) as e:
            z.run([1, 0], [], [])
        self.assertIn("14.3", e.exception.diag["section"])

    def test_superposition_output_rejects(self):
        p = compile_action(act("h t\n"), "a", [], 0)
        with self.assertRaises(Reject):
            p.run([0, 0], [], [])

    def test_rotation_is_unchecked_not_guessed(self):
        with self.assertRaises(Unchecked):
            compile_action(act("rz 0.7 t\n"), "a", [], 0)

    def test_dirty_free_rejects_with_T3(self):
        p = compile_action(act("d = qalloc any 1\ncx c d\nqfree d\n"), "a", [], 0)
        self.assertEqual(p.run([0, 0], [], []), ([0, 0], [], []))
        with self.assertRaises(Reject):
            p.run([0, 1], [], [])

    def test_control_inherited_keeps_allocations_unconditional(self):
        p = compile_action(act("d = qalloc any 1\nx d\nx d\nqfree d\ncx c t\n"), "a", [], 1)
        self.assertEqual(p.run([0, 0, 1], [], []), ([0, 0, 1], [], []))
        self.assertEqual(p.run([1, 0, 1], [], []), ([1, 1, 1], [], []))

    def test_borrow_positions_follow_execution_order_under_adj(self):
        sub = ("\naction @s (t:wire mut) () () () ()\n^entry():\n"
               "b1 = qborrow any 1\nb2 = qborrow any 2\nqreturn b1\nqreturn b2\n")
        fwd = act("action_call @s (t) () () ()\n", extra=sub.lstrip("\n") and sub[1:], extra_sym=("s",))
        adj = act("adj action_call @s (t) () () ()\n", extra=sub[1:], extra_sym=("s",))
        self.assertEqual(compile_action(fwd, "a", [], 0).borrow_widths, [1, 2])
        self.assertEqual(compile_action(adj, "a", [], 0).borrow_widths, [2, 1])

    def test_loop_borrow_lifetimes_are_dynamic(self):
        o = act("qloop 3 {\nb = qborrow any 1\ncx c b[0]\ncx c b[0]\nqreturn b\n}\n", regs="regs {  } bounds {3}\n")
        self.assertEqual(compile_action(o, "a", [], 0).borrow_widths, [1, 1, 1])

    def test_borrow_not_restored_rejects(self):
        o = act("b = qborrow any 1\ncx c b[0]\nqreturn b\n")
        p = compile_action(o, "a", [], 0)
        self.assertEqual(p.run([0, 0], [1], []), ([0, 0], [1], []))
        with self.assertRaises(Reject):
            p.run([0, 1], [1], [])

    def test_qloop_while_flags(self):
        # §4.6: iteration i under c_i; c_1=cond[0], c_2=c_1&cond[1]
        o = act("qloop 2 while k {\nx t\n}\n", hdr="(t:wire mut) (k:wire[2] in) () () ()", regs="regs {  } bounds {2}\n")
        p = compile_action(o, "a", [], 0)
        for k, flips in ((0, 0), (1, 1), (2, 0), (3, 2)):
            self.assertEqual(p.run([0, k], [], []), ([flips % 2, k], [], []), k)

    def test_region_ops_and_local_image(self):
        hdr = "(v:wire[2] mut) (a:wire in) (m:region[1,2] mut) () ()"
        o = act("qexchm m a v\n", hdr=hdr)
        p = compile_action(o, "a", [], 0)
        self.assertEqual(p.run([1, 1], [], [[2, 3]]), ([3, 1], [], [[2, 1]]))
        self.assertEqual(p.run([1, 0], [], [[2, 3]]), ([2, 0], [], [[1, 3]]))
        q = act("d = qload m a\nqxor d v\nqunload m a d\n", hdr=hdr)
        self.assertEqual(compile_action(q, "a", [], 0).run([0, 1], [], [[2, 3]]), ([3, 1], [], [[2, 3]]))

    def test_local_region_must_be_restored(self):
        data = "\nsection data\nimg 01\n"
        hdr = "(t:wire mut) (a:wire in) () () ()"
        ok = act("qregion r 0 8 @img\nd = qload r ()\ncx d[0] t\nqunload r () d\n", hdr=hdr, extra=data, extra_sym=())
        p = compile_action(ok, "a", [], 0)
        self.assertEqual(p.run([0, 0], [], []), ([1, 0], [], []))
        bad = act("qregion r 0 8 @img\nv = qalloc any 8\nx v[3]\nqexchm r () v\nx v[0]\nx v[3]\nqfree v\n",
                  hdr=hdr, extra=data)
        with self.assertRaises(Reject):
            compile_action(bad, "a", [], 0).run([0, 0], [], [])
