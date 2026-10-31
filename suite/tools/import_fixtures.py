#!/usr/bin/env python3
"""Import the reference interpreter's pinned fixtures as neutral conformance cases.

    python3 -I suite/tools/import_fixtures.py --binary <bennett-isa-ref> \
        --fixtures <reference-interpreter/tests/fixtures> --source-repo <worktree>

Every expectation recorded in expect.json is what the binary actually printed
(hashes, check, run outputs). The PLAN below only chooses WHICH commands to run
and carries the hand-derived values from the Rust tests and the .comments
files as ASSERTIONS: if the binary disagrees with a hand derivation, the import
aborts instead of recording the disagreement as truth. Nothing is hand-typed
into expect.json.
"""
import argparse
import json
import math
import os
import re
import subprocess
import sys

STRIP_ENV = ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "BROWSER")
S = "$OBJECT"
H = 1 / math.sqrt(2)


class ImportError_(Exception):
    pass


def approx(v, tol=1e-12):
    return ("~", v, tol)


def dig(doc, path):
    """Follow a dotted path; dict keys may themselves contain dots (inc8.x)."""
    parts = path.split(".")
    while parts:
        if isinstance(doc, list):
            doc = doc[int(parts.pop(0))]
            continue
        for n in range(1, len(parts) + 1):
            key = ".".join(parts[:n])
            if key in doc:
                doc = doc[key]
                parts = parts[n:]
                break
        else:
            raise KeyError(path)
    return doc


def matches(got, want):
    if isinstance(want, tuple) and want[0] == "~":
        return isinstance(got, (int, float)) and abs(got - want[1]) <= want[2]
    if isinstance(want, int) and not isinstance(want, bool):
        return got == want or str(got) == str(want)
    return got == want


class Run:
    """One command; `expect` maps dotted paths of the output to hand values."""

    def __init__(self, cmd, expect=None, fail=None, extra_files=None):
        self.cmd, self.expect, self.fail = cmd, expect or {}, fail
        self.extra_files = extra_files or {}


def oracle(sym, inputs=None, budget=None, mem=None, args=(), expect=None, fail=None, sub="run-oracle", more=()):
    cmd = [sub, S, sym]
    if inputs is not None:
        cmd += ["--input", ",".join(str(i) for i in inputs)]
    if budget is not None:
        cmd += ["--budget", str(budget)]
    if mem:
        cmd += ["--mem", mem]
    for a in args:
        cmd += ["--arg", a]
    return Run(cmd + list(more), expect, fail)


def pc(code, payload, steps=None):
    e = {"code": code, "payload": payload}
    if steps is not None:
        e["steps"] = steps
    return e


def act(sym, flags, expect=None, adjoint=False, args=(), sub="run-action", fail=None):
    cmd = [sub, S, sym] + list(flags)
    for a in args:
        cmd += ["--arg", a]
    if adjoint:
        cmd.append("--adjoint")
    return Run(cmd, expect, fail)


def cc(sub, sym, args="", more=(), expect=None, fail=None, files=None):
    return Run([sub, S, sym, "--args", args] + list(more), expect, fail, files)


def dedup_clauses(*lists):
    out = []
    for l in lists:
        for c in l:
            if c not in out:
                out.append(c)
    return out


# ---------------------------------------------------------------- the plan

def plan_inc8():
    runs = []
    for b, (payload, code, steps) in (("3", (0, 0, 3)), ("2", (0, 1, 2)), ("0", (0, 1, 0))):
        runs.append(oracle("inc8", [255], b, expect=pc(code, payload, steps)))
    runs.append(oracle("inc8", [8], 3, expect=pc(0, 9, 3)))
    for x in (0, 127, 128, 254):
        runs.append(oracle("inc8", [x], 3, expect=pc(0, x + 1, 3)))
    runs.append(oracle("inc8", [256], fail=["§"]))
    runs.append(Run(["step-check", S, "inc8", "--exhaustive", "--budget", "4"],
                    {"cases": 256, "mode": "exhaustive"}))
    runs.append(Run(["step-check", S, "inc8", "--suite", "--budget", "4"],
                    {"cases": 256, "mode": "exhaustive"}))
    runs.append(Run(["step-check", S, "inc8", "--exhaustive", "--max-bits", "40", "--budget", "3"], {"cases": 256}))
    runs.append(oracle("inc8", [8], 3, sub="reverse", expect={
        "status": "REVERSED", "payload": 9, "code": 0, "steps": 3, "reverse_steps": 3,
        "equals_entry": True, "frames_clean": True, "first_difference": None,
        "state_after_reversal.pc": 0, "state_after_reversal.faulted": False,
        "state_after_reversal.counters": None,
        "state_after_reversal.locations.inc8.x": 8,
        "state_after_reversal.locations.inc8.one": 0,
        "state_after_reversal.locations.inc8.r": 0}))
    runs.append(oracle("inc8", [8], 3, sub="reverse", more=["--steps", "2"],
                       expect={"payload": 0, "code": 1, "steps": 2, "reverse_steps": 2, "equals_entry": True}))
    runs.append(oracle("inc8", [8], 2, sub="reverse", more=["--steps", "3"], fail=["§5.4"]))
    return runs


def plan_sum_db():
    db = "db=0102030405060708090a0b0c0d0e0f10"
    big = "1000,4080,512,64"
    runs = []
    for n in (0, 1, 3, 8, 16, 17, 23):
        if n <= 16:
            e = pc(0, n * (n + 1) // 2, 14 * n + 10)
        else:
            e = pc(7, 0, 13)
        runs.append(oracle("sum_db", [n], big, mem=db, expect=e))
    # stack, depth and step cutoffs at n=3 (Rust: seed_recursive_memory_interface_and_stack_boundaries)
    runs.append(oracle("sum_db", [3], "1000,4080,64,64", mem=db, expect=pc(0, 6)))
    runs.append(oracle("sum_db", [3], "1000,4080,63,64", mem=db, expect=pc(3, 0)))
    runs.append(oracle("sum_db", [3], "1000,4080,512,3", mem=db, expect=pc(3, 0)))
    runs.append(oracle("sum_db", [3], "1000,4080,512,4", mem=db, expect=pc(0, 6)))
    runs.append(oracle("sum_db", [3], "51,4080,512,64", mem=db, expect=pc(1, 0)))
    # council r4 literal cutoffs
    for n, steps, stack, depth, (p, c, st) in (
            (0, 9, 16, 1, (0, 1, 9)), (0, 10, 16, 1, (0, 0, 10)), (2, 38, 48, 3, (3, 0, 38)),
            (2, 37, 48, 3, (0, 1, 37)), (2, 38, 47, 3, (0, 3, 29))):
        runs.append(oracle("sum_db", [n], "%d,4080,%d,%d" % (steps, stack, depth), mem=db, expect=pc(c, p, st)))
    runs.append(Run(["step-check", S, "sum_db", "--samples", "40", "--budget", "300,4080,512,64"],
                    {"cases": 40, "mode": "sampled"}))
    runs.append(Run(["step-check", S, "sum_db", "--exhaustive", "--budget", "300,4080,512,64"], fail=["§17.13"]))
    return runs


ARITH_TABLE = [
    ("add", (255, 2), (1, 0)), ("sub", (0, 1), (255, 0)), ("mul", (128, 3), (128, 0)),
    ("and", (170, 240), (160, 0)), ("or", (170, 240), (250, 0)), ("xor", (170, 240), (90, 0)),
    ("shl", (1, 7), (128, 0)), ("shl", (1, 8), (0, 0)), ("lshr", (128, 7), (1, 0)),
    ("ashr", (128, 7), (255, 0)), ("ashr", (128, 8), (0, 0)), ("udiv", (255, 2), (127, 0)),
    ("urem", (255, 2), (1, 0)), ("sdiv", (249, 2), (253, 0)), ("srem", (249, 2), (255, 0)),
    ("udiv", (3, 0), (0, 5)), ("sdiv", (128, 255), (0, 6)), ("srem", (128, 255), (0, 6)),
    ("eq", (2, 2), (1, 0)), ("ne", (2, 2), (0, 0)), ("ult", (255, 0), (0, 0)), ("ule", (2, 2), (1, 0)),
    ("slt", (255, 0), (1, 0)), ("sle", (128, 128), (1, 0)), ("select", (2, 9), (2, 0)),
    ("casts", (128, 0), (255, 0)), ("parmov", (3, 9), (9, 0)), ("jump", (0, 0), (42, 0)),
    ("jump", (1, 9), (9, 0)), ("badjump", (0, 0), (0, 4)),
]


def plan_arithmetic():
    runs = [oracle(n, i, 100, expect=pc(c, p)) for n, i, (p, c) in ARITH_TABLE]
    runs += [oracle("fault%d" % c, [0, 0], 1, expect=pc(c, 0)) for c in range(11)]
    return runs


HEAP = ((("heap", 0x1234, 0), ("zeroalloc", 1, 0), ("afterfree", 0, 7), ("pinned", 0, 7),
         ("doublefree", 0, 7), ("badstore", 0, 7), ("outoftable", 0, 2), ("faultif", 0, 9)))


def plan_heap():
    runs = [oracle(n, [], "100,32,0,0", expect=pc(c, p)) for n, p, c in HEAP]
    runs.append(oracle("heap", [], "100,1,0,0", expect={"code": 2}))
    runs.append(oracle("heap", [], "100,33,0,0", fail=["§5.4"]))
    runs.append(Run(["step-check", S, "heap", "--exhaustive", "--budget", "100,32,0,0"], {"mode": "exhaustive"}))
    return runs


def plan_nested():
    runs = []
    for b in (0, 2, 3, 4):
        tk = "token:%d" % b
        runs.append(oracle("nested", [255], 20, args=[tk], expect=pc(0 if b >= 3 else 9, 0)))
        runs.append(oracle("cancel", [255], 20, args=[tk], expect=pc(0, 0)))
        runs.append(oracle("ignore", [255], 20, args=[tk], expect=pc(0, 0)))
        runs.append(oracle("nested", [8], 20, args=[tk], expect=pc(0, 9) if b >= 3 else pc(9, 0)))
    return runs


def plan_mutual():
    return [oracle("main", [], "20,0,32,8", expect=pc(0, 4, 6))]


def plan_tiny():
    runs = []
    for x in (0, 1):
        for byte in (0x00, 0x01, 0xfe, 0xff):
            for b in (3, 4, 7, 8, 9):
                if x == 0 and b >= 4:
                    e = pc(5, 0)
                elif x == 1 and b >= 8:
                    e = pc(0, byte & 1)
                else:
                    e = pc(1, 0)
                runs.append(oracle("tiny", [x], b, mem="db=%02x" % byte, expect=e))
    runs.append(Run(["step-check", S, "tiny", "--exhaustive", "--budget", "9"],
                    {"cases": 512, "input_and_memory_bits": 9, "mode": "exhaustive"}))
    return runs


X257 = 2 ** 200 + 3
Y257 = 2 ** 100 + 5


def plan_wide():
    runs = []
    xy = [X257, Y257]
    runs.append(oracle("div257", xy, "2,0,16,4", expect={"payload": 2 ** 100 - 5, "code": 0, "steps": 2}))
    runs.append(oracle("div257", xy, "1,0,16,4", expect={"payload": 0, "code": 1}))
    runs.append(oracle("add257", xy, "2,0,16,4", expect={"payload": (X257 + Y257) % 2 ** 257, "code": 0, "steps": 2}))
    runs.append(oracle("call257", [X257], "3,0,16,4", expect={"payload": X257, "code": 0, "steps": 3}))
    runs.append(oracle("call257", [X257], "3,0,4,4", expect={"code": 3, "payload": 0}))
    runs.append(oracle("call257", [X257], "3,0,5,4", expect={"payload": X257, "code": 0}))
    runs.append(oracle("call257", [X257], "3,0,16,1", expect={"payload": X257, "code": 0}))
    runs.append(oracle("fault257", [X257], "1,0,16,4", expect={"code": X257, "payload": 0, "steps": 1}))
    for x in (0, 1):
        runs.append(oracle("fault_small257", [x], "1,0,16,4", expect={"code": X257, "payload": 0, "steps": 1}))
    runs.append(Run(["step-check", S, "fault_small257", "--exhaustive", "--budget", "2,0,16,4"], {"cases": 2}))
    runs.append(oracle("add257", [3, 5], "2,0,16,4", expect={"payload": 8, "code": 0}))
    return runs


def plan_actions():
    runs = []
    runs.append(act("flip", ["--mut", "1", "--in", "2"], {"state": {"1101": [1, 0]}, "mut.0": 3, "in.0": 2}))
    runs.append(act("flip", ["--basis", "1001"], {"state": {"1101": [1, 0]}, "mut.0": 3}))
    runs.append(act("flip", ["--mut", "3", "--in", "2"], {"adjoint": True, "mut.0": 1, "state": {"1001": [1, 0]}}, adjoint=True))
    runs.append(act("memory", ["--mut", "1", "--in", "2"], {"mut.0": 1, "in.0": 2, "state": {"1001": [1, 0]}}))
    runs.append(act("loop", ["--mut", "0", "--in", "0"], {"mut.0": 1, "state": {"1000": [1, 0]}}))
    runs.append(act("bell", ["--mut", "0", "--in", "0"], {"wires": 4}))
    runs.append(act("bell", ["--mut", "0", "--in", "0"], {"wires": 4, "adjoint": True}, adjoint=True))
    runs.append(act("bell", ["--mut", "0", "--in", "0"], {
        "status": "REVERSED", "equals_entry": True, "first_difference": None,
        "forward_state.0000": [approx(H), approx(0)], "forward_state.1100": [approx(H), approx(0)],
        "state": {"0000": [1, 0]}}, sub="reverse"))
    for x in range(4):
        runs.append(oracle("lifted", [x], 5, expect=pc(0, x ^ 1)))
    runs.append(Run(["run-cc", S, "bell", "--args", ""], fail=["§3.4"]))
    return runs


def plan_control_flow():
    runs = []
    for p in range(8):
        ones = 0
        while ones < 3 and p & (1 << ones):
            ones += 1
        for t in (0, 1):
            runs.append(act("while3", ["--mut", str(t), "--in", str(p)],
                            {"mut.0": t ^ (ones & 1), "in.0": p, "wires": 4}))
    runs.append(act("while3", ["--mut", "0", "--in", "5"], {"mut.0": 1, "in.0": 5}, adjoint=True))
    # wire order (t, p); allocation-order amplitudes [0, 1/sqrt2, 1/2, 1/2] are 00, 10, 01, 11
    runs.append(act("conditional", ["--state", "00=0.7071067811865475,0;01=0.7071067811865475,0"], {
        "state.10": [approx(H), approx(0)], "state.01": [approx(0.5), approx(0)],
        "state.11": [approx(0.5), approx(0)], "wires": 2}))
    runs.append(act("conditional", ["--state", "01=0.5,0;10=0.7071067811865475,0;11=0.5,0"], {
        "state.00": [approx(H), approx(0)], "state.01": [approx(H), approx(0)]}, adjoint=True))
    runs.append(act("minus_i", ["--mut", "0"], {"state": {"0": [-1, 0]}}))
    runs.append(act("minus_i", ["--mut", "1"], {"state": {"1": [-1, 0]}}))
    runs.append(act("minus_i", ["--mut", "1"], {"state": {"1": [-1, 0]}}, adjoint=True))
    runs.append(act("phase", ["--mut", "0"], {"wires": 1, "state.0": [approx(-1.0), approx(0.0)]}))
    runs.append(act("controlled_phase", ["--state", "00=0.7071067811865475,0;01=0.7071067811865475,0"], {
        "state.00": [approx(H, 1e-12), approx(0)], "state.01": [approx(-H, 1e-12), approx(0)]}))
    runs.append(act("exchange", ["--mut", "2", "--in", "1", "--mem", "m=0103"],
                    {"mut.0": 3, "mem.m": [1, 2], "state": {"1111001": [1, 0]}}))
    runs.append(act("exchange", ["--mut", "2", "--in", "1"], {"mut.0": 0, "mem.m": [0, 2]}))
    for p, want in ((0, 0xeb), (1, 0x98)):
        runs.append(act("image", ["--mut", "53", "--in", str(p)], {"mut.0": want, "in.0": p}))
    runs.append(act("borrowed", ["--mut", "0", "--in", "1"], {"mut.0": 1, "state": {"11": [1, 0]}}))
    for t in (0, 1):
        for x in (0, 1):
            runs.append(act("borrowed", ["--mut", str(t), "--in", str(x)], {"mut.0": t ^ x, "in.0": x}))
    return runs


def plan_classical():
    runs = []
    runs.append(cc("run-cc", "sum", "3", expect={
        "status": "COMPLETED", "steps": 21, "fault": None, "results.0.type": "i8", "results.0.value": "6",
        "trace.0.op": "const.i8", "trace.0.function": "sum", "trace.0.block": "entry", "trace.20.op": "ret"}))
    runs.append(Run(["run-cc", S, "sum", "--input", "3"], {"status": "COMPLETED", "steps": 21}))
    for n in (0, 1, 5, 8):
        runs.append(cc("run-cc", "sum", str(n), expect={
            "status": "COMPLETED", "results.0.value": str(n * (n + 1) // 2)}))
        runs.append(cc("reverse", "sum", str(n), expect={"status": "REVERSED", "equals_entry": True}))
    runs.append(cc("reverse", "sum", "3", more=["--steps", "5"], expect={
        "status": "REVERSED", "steps": 5, "fault": None, "equals_entry": True}))
    ent = ["--entropy", "0b1621"]
    runs.append(cc("run-cc", "effects", more=ent, expect={
        "status": "COMPLETED", "results.0.value": "0", "console_hex": "0b1621", "clock": 1,
        "effect_tape_length": 5, "tape.0.dev": "entropy", "tape.0.op": "read", "tape.0.bytes": "0b1621",
        "tape.0.args": [16, 3], "tape.1.dev": "console", "tape.2.dev": "clock",
        "tape.2.bytes": "0000000000000000", "tape.3.dev": "block",
        "trace.3.op": "effect", "trace.3.memory.0.address": 16, "trace.3.memory.0.pre": 0,
        "trace.3.memory.0.post": 11, "trace.3.memory.1.post": 22, "trace.3.memory.2.post": 33}))
    runs.append(cc("reverse", "effects", more=ent, expect={
        "status": "REVERSED", "steps": 11, "results.0.value": "0", "equals_entry": True,
        "memory_equals_entry": True, "first_difference": None, "console": "", "clock": 0,
        "effect_tape_length": 0}))
    runs.append(cc("run-cc", "nested", more=ent, expect={
        "status": "COMPLETED", "results.0.value": "0", "clock": 0, "effect_tape_length": 0}))
    runs.append(cc("reverse", "nested", more=ent, expect={"status": "REVERSED", "equals_entry": True}))
    for name, code in (("impure", 4), ("entropy_short", 10), ("unknown_effect", 8), ("memfault", 7), ("divzero", 5)):
        exp = {"status": "FAILED", "fault": code, "faults.0.code": code}
        if name == "entropy_short":
            exp["effect_tape_length"] = 0
        if name == "divzero":
            exp.update({"faults.0.name": "FAULT_UB_DIV0", "steps": 3})
        runs.append(cc("run-cc", name, expect=exp))
        runs.append(cc("reverse", name, expect={"equals_entry": True}))
    replies = '[{"payload":"a50000","reply":"1122334455","service":"org.bennett.test"}]'
    runs.append(cc("run-cc", "host", more=["--replies", "$CASE/replies-host.json"], files={"replies-host.json": replies}, expect={
        "results.0.type": "i16", "results.0.value": "5", "host_requests.0.service": "org.bennett.test",
        "host_requests.0.payload": "a50000", "host_requests.0.reply": "1122334455",
        "host_requests.0.address": 16, "host_requests.0.capacity": 3}))
    runs.append(cc("reverse", "host", more=["--replies", "$CASE/replies-host.json"], expect={"equals_entry": True, "memory_equals_entry": True}))
    wrong = '[{"payload":"00","reply":"","service":"org.bennett.test"}]'
    runs.append(cc("run-cc", "host", more=["--replies", "$CASE/replies-host-wrong.json"], files={"replies-host-wrong.json": wrong}, fail=["§10"]))
    runs.append(cc("run-cc", "host", fail=["HOST_REQUEST"]))
    runs.append(cc("run-cc", "host_invalid", expect={"status": "FAILED", "fault": 7}))
    runs.append(cc("run-cc", "sum", "256", fail=["E_ARG"]))
    runs.append(cc("run-cc", "sum", "", fail=["E_ARG"]))
    runs.append(cc("run-cc", "missing", "", fail=["§3.4"]))
    runs.append(Run(["run-cc", S, "sum", "--args", "3", "--input", "3"], fail=["--args"]))
    runs.append(Run(["run-cc", S, "sum", "--args", "3", "--steps", "2"], fail=["--steps is a reverse option"]))
    runs.append(cc("reverse", "missing", fail=["§"]))
    return runs


def plan_registers():
    runs = []
    runs.append(act("bell", ["--mut", "0"], {
        "action": "bell", "wires": 2, "state.00": [approx(H), approx(0)], "state.11": [approx(H), approx(0)],
        "mut.0": None}))
    runs.append(act("toggle", ["--mut", "0"], {"state": {"1": [1, 0]}, "mut.0": 1}))
    runs.append(act("bell", ["--state", "00=0.7071067811865475,0;11=0.7071067811865475,0"], {
        "state.00": [approx(1.0), approx(0)], "mut.0": 0}, adjoint=True))
    bell = S
    runs.append(Run(["run-action", bell, "bell", "--mut", "0", "--basis", "00"], fail=["exactly one of --mut, --basis, --state"]))
    runs.append(Run(["run-action", bell, "bell"], fail=["exactly one of --mut, --basis, --state"]))
    runs.append(Run(["run-action", bell, "bell", "--basis", "00", "--in", "0"], fail=["--in accompanies --mut only"]))
    runs.append(Run(["run-action", bell, "bell", "--basis", "000"], fail=["§14.2"]))
    runs.append(Run(["run-action", bell, "bell", "--state", "00=0,0"], fail=["§14.2"]))
    runs.append(Run(["run-action", bell, "missing", "--mut", "0"], fail=["§5.3"]))
    runs.append(cc("run-program", "allocation", expect={"status": "COMPLETED", "run_state": "CLOSED", "fault": None}))
    runs.append(cc("run-program", "allocation", more=["--reverse"], expect={"status": "REVERSED", "equals_entry": True}))
    runs.append(cc("run-program", "views", expect={
        "status": "COMPLETED", "run_state": "CLOSED", "reversed": False, "steps": 7, "results": [], "faults": [],
        "fault": None, "measurements": [], "budgets": []}))
    runs.append(cc("run-program", "views", more=["--reverse"], expect={
        "status": "REVERSED", "reversed": True, "memory_equals_entry": True, "equals_entry": True}))
    runs.append(cc("run-program", "unclean", expect={"status": "FAILED", "steps": 3, "fault": 4, "results": []}))
    runs.append(cc("run-program", "unclean", more=["--reverse"], fail=["E_INVALID_STATE"]))
    for draw, bits, val in (("0.1", "00", 0), ("0.9", "11", 3)):
        runs.append(cc("run-program", "measurement", more=["--draws", draw], expect={
            "status": "COMPLETED", "steps": 4, "results.0.type": "i2", "results.0.value": str(val),
            "measurements.0.reg": 1, "measurements.0.bits": bits, "measurements.0.value": val}))
    runs.append(cc("run-program", "measurement", more=["--draws", "0.1", "--reverse"], fail=["E_NOT_REVERSIBLE"]))
    runs.append(cc("reverse", "measurement", more=["--draws", "0.1"], fail=["E_NOT_REVERSIBLE"]))
    runs.append(cc("run-cc", "bad_uncall", more=["--draws", "0.1"], expect={"status": "FAILED", "fault": 4}))
    runs.append(cc("run-cc", "unclean", expect={"status": "FAILED", "fault": 4}))
    return runs


def r9_runs(tokens):
    runs = []
    for token in tokens:
        for flag, inp in (("--mut", "0"), ("--mut", "1"), ("--state", "0=0.6,0;1=0,0.8")):
            for adj in (False, True):
                e = {"wires": 1}
                if flag == "--mut":
                    e["state"] = {inp: [1, 0]}
                else:
                    e.update({"state.0": [approx(0.6), approx(0.0)], "state.1": [approx(0.0), approx(0.8)]})
                runs.append(act("a", [flag, inp], e, adjoint=adj, args=[token] if token else []))
            runs.append(act("a", [flag, inp], {"equals_entry": True, "wires": 1}, sub="reverse", args=[token] if token else []))
    return runs


def plan_r9_sol_oracle():
    return r9_runs(["token:0,0,0,0", "token:1,0,0,0"])


def plan_r9_codex_oracle():
    return r9_runs(["token:0,0,0,0", "token:1,0,0,0"])


def plan_r9_sol_qinplace():
    return r9_runs(["token:1,0,0,0"]) + [act("a", ["--mut", "0"], fail=["§4.5"], args=["token:0,0,0,0"])]


def plan_r9_loop():
    return r9_runs([""])


# case -> (plan, curated clauses, pins). pins: independent hashes pinned elsewhere
# in the Rust tests (hashlib-derived); the import aborts if the binary disagrees.
PLAN = {
    "inc8": (plan_inc8, ["ISA-16.3", "ISA-3.6", "ISA-4.2", "ISA-5.4", "ISA-14.1", "ISA-9.2"], {}),
    "sum_db": (plan_sum_db, ["ISA-16.3", "ISA-3.7", "ISA-5.4", "ISA-7.2", "ISA-14.1", "ISA-17"], {}),
    "arithmetic": (plan_arithmetic, ["ISA-3.3", "ISA-3.6", "ISA-4.2", "ISA-5.4"], {}),
    "heap": (plan_heap, ["ISA-4.2", "ISA-5.4", "ISA-7.2"], {}),
    "nested": (plan_nested, ["ISA-4.2", "ISA-5.2", "ISA-5.4", "ISA-5.5", "ISA-3.7"], {}),
    "mutual": (plan_mutual, ["ISA-3.7", "ISA-4.2", "ISA-9.2"],
               {"f": "e4028cf7347c2779e19c8193ef1039837944acebd4784e9c3a42fbe236e737ec",
                "g": "7dc9c25aff5230a46ae697edf187d34ba72a03f7a3c1a520c6fc6bf5462c6374"}),
    "tiny_memory": (plan_tiny, ["ISA-3.6", "ISA-5.4", "ISA-7.2", "ISA-14.1"], {}),
    "wide": (plan_wide, ["ISA-3.3", "ISA-3.6", "ISA-3.7", "ISA-5.4", "ISA-14.1"], {}),
    "actions": (plan_actions, ["ISA-3.4", "ISA-4.2", "ISA-4.5", "ISA-4.6", "ISA-5.3", "ISA-10", "ISA-14.2"], {}),
    "control_flow": (plan_control_flow, ["ISA-3.5", "ISA-4.4", "ISA-4.6", "ISA-5.3", "ISA-5.6", "ISA-14.2"], {}),
    "classical": (plan_classical, ["ISA-3.4", "ISA-4.1", "ISA-5.4", "ISA-7.1", "ISA-7.3", "ISA-7.4", "ISA-10"], {"g": "e8a3dc5429049b16559aa6a4262779e3dcd37a758967def4e08c371a5b765fff"}),
    "registers": (plan_registers, ["ISA-3.1", "ISA-3.3", "ISA-4.7", "ISA-7.4", "ISA-10", "ISA-14.2"], {}),
    "r9-sol-oracle": (plan_r9_sol_oracle, ["ISA-5.6", "ISA-6.4", "ISA-9.2"], {"@object": "ef010633dbc984abbaa9fbfb97c6233c5390fa99a17bc38e09725cddec936832"}),
    "r9-codex-oracle": (plan_r9_codex_oracle, ["ISA-5.6", "ISA-6.4", "ISA-9.2"], {"@object": "49ad3ba6c411ed73bebacdebe6728ebff5995dbf8bbc03c016d1cb0c5c3e8625"}),
    "r9-sol-qinplace": (plan_r9_sol_qinplace, ["ISA-4.5", "ISA-5.6", "ISA-6.4", "ISA-9.2"], {"@object": "77664ac9392aec50b0415027771ae7ef49dcea7762b70025f0cd918d53a6ee1e"}),
    "r9-loop": (plan_r9_loop, ["ISA-5.6", "ISA-6.4", "ISA-9.2"], {"@object": "80478acc73d52bbf64c395816470293b5b4b4a1ee36a5c550bf00436f06fc192"}),
}

# ---------------------------------------------------------------- machinery


def run_binary(binary, args, cwd):
    env = {k: v for k, v in os.environ.items() if k not in STRIP_ENV}
    p = subprocess.run([binary] + args, cwd=cwd, env=env, capture_output=True, timeout=600)
    return p.returncode, p.stdout.decode(), p.stderr.decode()


def assert_subset(case, run, doc):
    for path, want in run.expect.items():
        try:
            got = dig(doc, path)
        except (KeyError, IndexError, ValueError, TypeError):
            if isinstance(want, dict):
                got = None
            else:
                raise ImportError_("%s %s: path %s missing in output" % (case, run.cmd, path))
        if isinstance(want, dict):
            sub = dig(doc, path) if path else doc
            if path and not isinstance(sub, dict):
                raise ImportError_("%s %s: %s not an object" % (case, run.cmd, path))
            for k, v in want.items():
                if k not in sub or not all_match(sub[k], v):
                    raise ImportError_("%s %s: %s.%s = %r, hand value %r" % (case, run.cmd, path, k, sub.get(k), v))
            if set(sub) != set(want):
                raise ImportError_("%s %s: %s keys %s != hand-derived %s" % (case, run.cmd, path, sorted(sub), sorted(want)))
            continue
        if want is None and path.endswith("counters"):
            continue
        if not all_match(got, want):
            raise ImportError_("%s %s: %s = %r, hand value %r" % (case, run.cmd, path, got, want))


def all_match(got, want):
    if isinstance(want, list):
        return isinstance(got, list) and len(got) == len(want) and all(all_match(g, w) for g, w in zip(got, want))
    return matches(got, want)


def import_case(a, name, fixtures, out, source):
    plan_fn, curated, pins = PLAN[name]
    case_dir = os.path.join(out, name)
    os.makedirs(case_dir, exist_ok=True)
    with open(os.path.join(fixtures, name + ".bisa"), "rb") as f:
        obj = f.read()
    with open(os.path.join(case_dir, "object.bisa"), "wb") as f:
        f.write(obj)
    with open(os.path.join(fixtures, name + ".comments"), "rb") as f:
        notes = f.read()
    with open(os.path.join(case_dir, "notes.md"), "wb") as f:
        f.write(notes)
    object_path = os.path.join(case_dir, "object.bisa")

    code, hout, err = run_binary(a.binary, ["hash", object_path], case_dir)
    if code != 0:
        raise ImportError_("%s: hash failed: %s" % (name, err))
    h = json.loads(hout)
    code, cout, err = run_binary(a.binary, ["check", object_path], case_dir)
    if code != 0 or json.loads(cout).get("status") != "ok":
        raise ImportError_("%s: check not ok: %s" % (name, err))
    if json.loads(cout) != h:
        raise ImportError_("%s: hash and check disagree" % name)
    for k, v in pins.items():
        got = h["object_hash"] if k == "@object" else h["symbols"][k]["interface_hash"]
        if got != v:
            raise ImportError_("%s: %s hash %s != Rust-pinned %s" % (name, k, got, v))
    hashes = {"object": h["object_hash"], "symbols": {
        n: {"interface": s["interface_hash"], "closure": s["closure_hash"]} for n, s in sorted(h["symbols"].items())}}

    runs = []
    for run in plan_fn():
        for fname, content in run.extra_files.items():
            with open(os.path.join(case_dir, fname), "w") as f:
                f.write(content)
        code, sout, serr = run_binary(a.binary, [x.replace("$OBJECT", object_path).replace("$CASE", case_dir) for x in run.cmd], case_dir)
        rec = {"cmd": run.cmd, "exit": code}
        if run.fail is not None:
            if code != 1:
                raise ImportError_("%s %s: expected a diagnostic (exit 1), got %d: %s" % (name, run.cmd, code, serr))
            if sout:
                raise ImportError_("%s %s: diagnostic printed partial stdout" % (name, run.cmd))
            for n in run.fail:
                if n not in serr:
                    raise ImportError_("%s %s: stderr %r lacks %r" % (name, run.cmd, serr, n))
            rec["stdout"] = ""
            rec["stderr_contains"] = list(run.fail)
        else:
            if code != 0:
                raise ImportError_("%s %s: exit %d: %s" % (name, run.cmd, code, serr))
            doc = json.loads(sout)
            assert_subset(name, run, doc)
            rec["stdout_json"] = doc
        runs.append(rec)

    text = notes.decode()
    from_comments = sorted(set("ISA-" + m for m in re.findall(r"§(\d+(?:\.\d+)?)", text)))
    clauses = dedup_clauses(curated, from_comments)
    expect = {"spec": "isa-0.8", "source": source, "clauses": clauses, "hashes": hashes, "check": "ok", "runs": runs}
    with open(os.path.join(case_dir, "expect.json"), "w") as f:
        json.dump(expect, f, indent=1, ensure_ascii=False)
        f.write("\n")
    return len(runs)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", required=True)
    ap.add_argument("--fixtures", required=True)
    ap.add_argument("--source-repo", required=True, help="checkout the binary was built from")
    ap.add_argument("--out", default=os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "cases"))
    a = ap.parse_args()
    a.binary = os.path.abspath(a.binary)
    a.out = os.path.abspath(a.out)
    st = subprocess.run(["git", "-C", a.source_repo, "status", "--porcelain", "--", "reference-interpreter"],
                        capture_output=True, text=True, check=True).stdout.strip()
    if st:
        raise SystemExit("source repo has uncommitted changes under reference-interpreter/:\n" + st)
    head = subprocess.run(["git", "-C", a.source_repo, "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    source = "reference-interpreter@" + head
    fixtures = sorted(f[:-5] for f in os.listdir(a.fixtures) if f.endswith(".bisa"))
    if sorted(fixtures) != sorted(PLAN):
        raise SystemExit("fixture set %s != plan %s" % (fixtures, sorted(PLAN)))
    total = 0
    for name in fixtures:
        try:
            n = import_case(a, name, a.fixtures, a.out, source)
        except ImportError_ as err:
            raise SystemExit("IMPORT ABORTED: %s" % err)
        print("%-18s %3d runs" % (name, n))
        total += n
    print("imported %d cases, %d runs, source %s" % (len(fixtures), total, source))


if __name__ == "__main__":
    main()
