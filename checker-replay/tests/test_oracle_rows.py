"""Row semantics from §4.1/§4.2/§3.6/§3.7/§5.4/§7.2, each with a hand-derived expectation."""
import unittest
from checker_replay.objparse import parse_object
from checker_replay.oracle import oracle_payload

HEAD = "bennett-object isa=0.9 object=0.9 launch=0.9 sched=0.9 cert=0.9 coverage=0.9 target=0.9 ptr_width=8\n"


def obj(fns, layout=None, data=None, sym=None):
    import json
    syms = sym or {"0": {"interface_hash": "0" * 64, "kind": "oracle", "name": "o",
                         "signature": {"abi": "0.9", "args": [], "fault": 4, "in": [8, 8], "kind": "oracle",
                                       "mem": [], "tgt": 8, "tier": "q"}}}
    t = HEAD + "\nsection symbols\n" + json.dumps(syms) + "\n\nsection functions\n" + fns
    if data:
        t += "\nsection data\n" + data
    if layout:
        t += "\nsection layout\n" + json.dumps(layout) + "\n"
    return parse_object(t)


def binop(op, a, b, lhs_w=8):
    src = ("oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"
           f"regs {{ r:reg[{lhs_w}] }}\n^entry():\nr = {op} a b\nlift.retval r\n")
    return oracle_payload(obj(src), "o", [a, b], [], [], (5, 0, 0, 0))


class Binops(unittest.TestCase):
    def test_wrap(self):
        self.assertEqual(binop("lift.add", 200, 100), (44, 0))
        self.assertEqual(binop("lift.sub", 1, 2), (255, 0))
        self.assertEqual(binop("lift.mul", 16, 16), (0, 0))

    def test_shift_at_or_above_width_is_zero_all_three(self):
        # §4.1 "shift >= N -> 0", no sign exception for ashr
        for op in ("lift.shl", "lift.lshr", "lift.ashr"):
            self.assertEqual(binop(op, 0x80, 8), (0, 0), op)
            self.assertEqual(binop(op, 0xFF, 200), (0, 0), op)

    def test_ashr_sign_fill_below_width(self):
        self.assertEqual(binop("lift.ashr", 0x80, 7), (0xFF, 0))
        self.assertEqual(binop("lift.lshr", 0x80, 7), (1, 0))

    def test_div_faults_and_truncation(self):
        self.assertEqual(binop("lift.udiv", 7, 0), (0, 5))
        self.assertEqual(binop("lift.srem", 7, 0), (0, 5))
        self.assertEqual(binop("lift.sdiv", 0x80, 0xFF), (0, 6))
        self.assertEqual(binop("lift.srem", 0x80, 0xFF), (0, 6))
        self.assertEqual(binop("lift.sdiv", 0xF9, 2), (0xFD, 0))  # -7/2 = -3 toward zero
        self.assertEqual(binop("lift.srem", 0xF9, 2), (0xFF, 0))  # -7 rem 2 = -1


class Control(unittest.TestCase):
    def test_exactly_B_rows_budget_and_retval_on_last_step(self):
        src = ("oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"
               "regs { r:reg[8] }\n^entry():\nr = lift.mov a\nlift.retval r\n")
        self.assertEqual(oracle_payload(obj(src), "o", [9, 0], [], [], (2, 0, 0, 0)), (9, 0))
        self.assertEqual(oracle_payload(obj(src), "o", [9, 0], [], [], (1, 0, 0, 0)), (0, 1))

    def test_fault_zero_effective_cause_is_trap(self):
        src = ("oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"
               "regs { p:reg[1], r:reg[8] }\n^entry():\np = lift.icmp.eq a b\nlift.faultif p 0\nr = lift.mov a\nlift.retval r\n")
        self.assertEqual(oracle_payload(obj(src), "o", [1, 1], [], [], (9, 0, 0, 0)), (0, 4))
        self.assertEqual(oracle_payload(obj(src), "o", [1, 2], [], [], (9, 0, 0, 0)), (1, 0))
        src2 = "oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n^entry():\nfault 9\n"
        self.assertEqual(oracle_payload(obj(src2), "o", [1, 1], [], [], (9, 0, 0, 0)), (0, 9))

    def test_label_and_qjumpr_canonical_index(self):
        # rows: 0 w=label ^b, 1 qjumpr w, 2 fault, 3 retval w (index of ^b is 3)
        src = ("oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"
               "regs { w:reg[8] }\n^entry():\nw = lift.label ^b\nqjumpr w\n^c():\nfault 9\n^b():\nlift.retval w\n")
        self.assertEqual(oracle_payload(obj(src), "o", [0, 0], [], [], (9, 0, 0, 0)), (3, 0))

    def test_qjumpr_out_of_range_traps(self):
        src = ("oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"
               "^entry():\nqjumpr a\n^c():\nlift.retval a\n")
        self.assertEqual(oracle_payload(obj(src), "o", [1, 0], [], [], (9, 0, 0, 0)), (1, 0))
        self.assertEqual(oracle_payload(obj(src), "o", [2, 0], [], [], (9, 0, 0, 0)), (0, 4))

    def test_parmov_is_parallel(self):
        src = ("oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"
               "regs { x:reg[8], y:reg[8], r:reg[8] }\n^entry():\nx = lift.mov a\ny = lift.mov b\n"
               "(x, y) = lift.parmov (y, x)\nr = lift.sub x y\nlift.retval r\n")
        self.assertEqual(oracle_payload(obj(src), "o", [1, 5], [], [], (9, 0, 0, 0)), (4, 0))


LAYOUT = {"A_tab": 2, "H": 8, "S_cells": 4, "globals": {"g": {"base": 4, "image": "g", "size": 2}}, "heap_base": 16,
          "heap_limit_max": 200, "interface_regions": {}, "null_size": 4, "ptr_width": 8}


class Memory(unittest.TestCase):
    PRE = "oracle @o (a:wire[8] in, b:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"

    def run_(self, body, regs, budget=(99, 100, 0, 0), a=0, b=0, layout=LAYOUT):
        sym = {"0": {"interface_hash": "0" * 64, "kind": "oracle", "name": "o",
                     "signature": {"abi": "0.9", "args": [], "fault": 4, "in": [8, 8], "kind": "oracle",
                                   "mem": [], "tgt": 8, "tier": "q"}},
               "1": {"interface_hash": "0" * 64, "kind": "data", "name": "g",
                     "signature": {"abi": "0.9", "kind": "data", "size": 2, "tier": "-"}}}
        src = self.PRE + f"regs {{ {regs} }}\n^entry():\n{body}"
        return oracle_payload(obj(src, layout=layout, data="g 0a0b\n", sym=sym), "o", [a, b], [], [], budget)

    def test_global_image_little_endian_16(self):
        self.assertEqual(self.run_("p = lift.addr @g\nv = lift.load.i16 p\nr = lift.trunc.i8 v\nlift.retval r\n",
                                   "p:reg[8], v:reg[16], r:reg[8]"), (0x0a, 0))
        self.assertEqual(self.run_("p = lift.addr @g\nv = lift.load.i16 p\ns = lift.const.i16 8\nq = lift.lshr v s\nr = lift.trunc.i8 q\nlift.retval r\n",
                                   "p:reg[8], v:reg[16], q:reg[16], s:reg[16], r:reg[8]"), (0x0b, 0))

    def test_load_straddling_entry_end_memfaults(self):
        self.assertEqual(self.run_("c = lift.const.i8 5\nv = lift.load.i16 c\nlift.retval v\n",
                                   "c:reg[8], v:reg[16]"), (0, 7))

    def test_alloc_zero_bumps_one_and_free_rules(self):
        body = ("n = lift.const.i8 0\np = lift.alloc n\nq = lift.alloc n\nd = lift.sub q p\n"
                "lift.free q\nlift.retval d\n")
        self.assertEqual(self.run_(body, "n:reg[8], p:reg[8], q:reg[8], d:reg[8]"), (1, 0))

    def test_double_free_and_free_pinned_memfault(self):
        self.assertEqual(self.run_("n = lift.const.i8 1\np = lift.alloc n\nlift.free p\nlift.free p\nlift.retval n\n",
                                   "n:reg[8], p:reg[8]"), (0, 7))
        self.assertEqual(self.run_("p = lift.addr @g\nlift.free p\nlift.retval p\n", "p:reg[8]"), (0, 7))

    def test_alloc_heap_limit_and_table_limit(self):
        self.assertEqual(self.run_("n = lift.const.i8 5\np = lift.alloc n\nlift.retval p\n", "n:reg[8], p:reg[8]",
                                   budget=(9, 4, 0, 0)), (0, 2))
        self.assertEqual(self.run_("n = lift.const.i8 5\np = lift.alloc n\nlift.retval p\n", "n:reg[8], p:reg[8]",
                                   budget=(9, 5, 0, 0)), (16, 0))
        # A_tab=2: 4 entries, global is pre-entry 0, so the 4th alloc overflows
        body = "n = lift.const.i8 1\n" + "p = lift.alloc n\n" * 4 + "lift.retval p\n"
        self.assertEqual(self.run_(body, "n:reg[8], p:reg[8]"), (0, 2))

    def test_store_then_load_and_boundscheck(self):
        self.assertEqual(self.run_("n = lift.const.i8 2\np = lift.alloc n\nlift.store.i8 p a\nv = lift.load.i8 p\n"
                                   "lift.boundscheck p 2\nlift.retval v\n", "n:reg[8], p:reg[8], v:reg[8]", a=77), (77, 0))
        self.assertEqual(self.run_("n = lift.const.i8 2\np = lift.alloc n\nlift.boundscheck p 3\nlift.retval p\n",
                                   "n:reg[8], p:reg[8]"), (0, 7))


class Nested(unittest.TestCase):
    def test_nested_oracle_uses_own_token_and_xors_fault(self):
        import json
        syms = {"0": {"interface_hash": "0" * 64, "kind": "oracle", "name": "o",
                      "signature": {"abi": "0.9", "args": ["token"], "fault": 4, "in": [8], "kind": "oracle",
                                    "mem": [], "tgt": 8, "tier": "q"}},
                "1": {"interface_hash": "0" * 64, "kind": "oracle", "name": "c",
                      "signature": {"abi": "0.9", "args": [], "fault": 4, "in": [8], "kind": "oracle",
                                    "mem": [], "tgt": 8, "tier": "q"}}}
        src = ("oracle @o (a:wire[8] in) (t:wire[8]) (f:wire[4]) () (tk:token)\nregs { x:reg[8], g:reg[4], r:reg[8] }\n"
               "^entry():\nlift.oracle_call @c tk (a) x g () ()\nr = lift.zext.i8 g\nr = lift.xor r x\nlift.retval r\n"
               "oracle @c (a:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\nregs { r:reg[8] }\n^entry():\nr = lift.mov a\nlift.retval r\n")
        o = obj(src, sym=syms)
        self.assertEqual(oracle_payload(o, "o", [5], [], [("2", "0", "0", "0")], (9, 0, 0, 0)), (5, 0))
        # callee budget 1: not halted -> (0, FAULT_BUDGET_STEPS), the outer B unaffected
        self.assertEqual(oracle_payload(o, "o", [5], [], [("1", "0", "0", "0")], (5, 0, 0, 0)), (1, 0))
