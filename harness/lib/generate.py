"""Random well-formed ISA programs by tier, seeded and deterministic (stream: prng.Prng).

Classes: r (reversible tier: arithmetic, memory, effects, segments), lift (oracle rows and lprocs),
cc (classical control with registers, measurement, region calls), q (structured actions).
Well-formedness follows the spec's rules: §3.3 width and linearity, §4 forms, §5.1, §5.3 bounds, §6 windows,
§9 metadata (identity recomputed by identity.py). Faults at run time are intended behaviour, not rejections.
Hostcalls are not generated (replies depend on the path); corpora cover them, and docs/harness.md says so.
Shrinking is shrink.py: reductions keep identity and the divergence signature.
"""
import concurrent.futures
import time

import engine
import identity
import objfile
from prng import Prng

CLASSES = ("r", "lift", "cc", "q", "mutant")
WELL_FORMED = ("r", "lift", "cc", "q")
WIDTHS = (1, 2, 7, 8, 16, 32, 63, 64, 65, 128)
MEM_WIDTHS = (8, 16, 32, 64, 128)
BINOPS = ("add", "sub", "mul", "and", "or", "xor", "shl", "lshr", "ashr")
DIVOPS = ("udiv", "urem", "sdiv", "srem")
PREDS = ("eq", "ne", "ult", "ule", "slt", "sle")
DRAWS = ("0.0", "0.25", "0.5", "0.75", "0.875", "0.999")


class Program:
    def __init__(self, text, invocations):
        self.text, self.invocations = text, invocations  # invocations: list of (argv, tape?)


def _header(ptr_width, isa="0.8"):
    return ("bennett-object isa=%s object=%s launch=%s sched=%s cert=%s coverage=%s target=%s ptr_width=%d\n"
            % (isa, isa, isa, isa, isa, isa, isa, ptr_width))


def _assemble(ptr_width, symbols, functions, data=(), devices=None, layout=None):
    table = {str(i): {"interface_hash": "", "kind": k, "name": n, "signature": s} for i, (n, k, s) in enumerate(symbols)}
    parts = [_header(ptr_width), "\nsection symbols\n", objfile.jcs(table), "\n\nsection functions\n",
             "\n".join("\n".join(f) for f in functions), "\n"]
    if data:
        parts += ["\nsection data\n", "".join("%s %s\n" % d for d in data)]
    if devices is not None:
        parts += ["\nsection devices\n", objfile.jcs(devices), "\n"]
    if layout is not None:
        parts += ["\nsection layout\n", objfile.jcs(layout), "\n"]
    return identity.rehash(objfile.parse("".join(parts))).text()


# ---------------------------------------------------------------- CC and R bodies

class ClassicalBody:
    """SSA builder for one CC/R function; structured control keeps dominance trivially right."""

    def __init__(self, rng, tier, ptr_width, helpers):
        self.rng, self.tier, self.ptr_width, self.helpers = rng, tier, ptr_width, helpers
        self.lines = ["^entry():"]
        self.n = 0
        self.values = []  # (name, width) of integers in scope
        self.ptrs = []
        self.ledger = []

    def fresh(self, stem="v"):
        self.n += 1
        return "%s%d" % (stem, self.n)

    def emit(self, line):
        self.lines.append(line)

    def const(self, width, value=None):
        name = self.fresh()
        v = self.rng.int_of_width(width) if value is None else value
        self.emit("%s = const.i%d %d" % (name, width, v))
        self.values.append((name, width))
        return name

    def value(self, width=None):
        cands = [n for n, w in self.values if width is None or w == width]
        if not cands or self.rng.chance(15):
            return self.const(width if width is not None else self.rng.pick(WIDTHS))
        return self.rng.pick(cands)

    def width_of(self, name):
        return next(w for n, w in self.values if n == name)

    def alloc(self):
        size = self.rng.pick((1, 2, 8, 16, 32, 64))
        n = self.const(self.ptr_width, size)
        p = self.fresh("p")
        self.emit("%s = alloc %s" % (p, n))
        self.ptrs.append(p)
        return p

    def ptr(self):
        if not self.ptrs or self.rng.chance(10):
            return self.alloc()
        return self.rng.pick(self.ptrs)

    def instruction(self):
        r = self.rng
        k = r.below(100)
        if k < 28:
            a = self.value()
            b = self.value(self.width_of(a))
            name = self.fresh()
            self.emit("%s = %s %s %s" % (name, r.pick(BINOPS), a, b))
            self.values.append((name, self.width_of(a)))
        elif k < 34:
            a = self.value()
            b = self.value(self.width_of(a))
            name = self.fresh()
            self.emit("%s = %s %s %s" % (name, r.pick(DIVOPS), a, b))
            self.values.append((name, self.width_of(a)))
        elif k < 42:
            a = self.value()
            b = self.value(self.width_of(a))
            name = self.fresh()
            self.emit("%s = icmp.%s %s %s" % (name, r.pick(PREDS), a, b))
            self.values.append((name, 1))
        elif k < 47:
            p = self.value(1)
            a = self.value()
            b = self.value(self.width_of(a))
            name = self.fresh()
            self.emit("%s = select %s %s %s" % (name, p, a, b))
            self.values.append((name, self.width_of(a)))
        elif k < 55:
            a = self.value()
            w = self.width_of(a)
            target = r.pick(WIDTHS)
            if target == w:
                return
            op = "trunc" if target < w else r.pick(("zext", "sext"))
            name = self.fresh()
            self.emit("%s = %s.i%d %s" % (name, op, target, a))
            self.values.append((name, target))
        elif k < 60:
            p = self.ptr()
            name = self.fresh()
            target = r.pick(WIDTHS)
            self.emit("%s = ptrtoint.i%d %s" % (name, target, p))
            self.values.append((name, target))
        elif k < 63:
            a = self.value()
            p = self.fresh("p")
            self.emit("%s = inttoptr %s" % (p, a))
            self.ptrs.append(p)
        elif k < 73:
            w = r.pick(MEM_WIDTHS)
            self.emit("store.i%d %s %s" % (w, self.ptr(), self.value(w)))
        elif k < 83:
            w = r.pick(MEM_WIDTHS)
            name = self.fresh()
            self.emit("%s = load.i%d %s" % (name, w, self.ptr()))
            self.values.append((name, w))
        elif k < 85 and self.ptrs:
            p = r.pick(self.ptrs)
            self.emit("free %s" % p)
            if r.chance(90):
                self.ptrs.remove(p)
        elif k < 93 and self.helpers:
            self.call()
        elif k < 94:
            self.emit("trap")
            self.lines.append("^%s():" % self.fresh("dead"))  # unreachable continuation keeps blocks well-formed
        else:
            self.alloc()

    def call(self):
        name, params, result = self.rng.pick(self.helpers)
        args = [self.value(w) for w in params]
        res = self.fresh("c")
        self.emit("%s = call @%s%s" % (res, name, "".join(" " + a for a in args)))
        self.values.append((res, result))
        if self.rng.chance(40):
            self.emit("uncall @%s (%s) (%s)" % (name, ", ".join(args), res))
            self.values.remove((res, result))

    def effect(self):
        r = self.rng
        kind = r.below(5)
        if kind == 0:
            p = self.alloc()
            self.emit("effect clock.read %s" % p)
        elif kind == 1:
            self.emit("effect console.write %s %s" % (self.ptr(), self.const(self.ptr_width, r.below(9))))
        elif kind == 2:
            self.emit("effect entropy.read %s %s" % (self.ptr(), self.const(self.ptr_width, r.below(9))))
        else:
            n = self.const(self.ptr_width, 512)
            p = self.fresh("p")
            self.emit("%s = alloc %s" % (p, n))
            self.ptrs.append(p)
            sector = self.const(64, r.below(4))
            self.emit("effect block.%s %s %s" % ("read" if kind == 3 else "write", sector, p))

    def statements(self, count, depth=0):
        for _ in range(count):
            k = self.rng.below(100)
            if k < 8 and depth < 2:
                self.diamond(depth)
            elif k < 14 and depth < 2:
                self.loop(depth)
            elif k < 28 and self.tier == "r":
                self.effect()
            else:
                self.instruction()

    def _scoped(self, body):
        saved = list(self.values), list(self.ptrs)
        body()
        self.values, self.ptrs = saved

    def diamond(self, depth):
        p = self.value(1)
        a = self.value()
        w = self.width_of(a)
        b = self.value(w)
        t, e, m, tv, ev, mv = (self.fresh(x) for x in ("then", "else", "merge", "tv", "ev", "mv"))
        self.emit("br %s ^%s(%s) ^%s(%s)" % (p, t, a, e, b))

        def arm(label, param):
            def go():
                self.lines.append("^%s(%s:i%d):" % (label, param, w))
                self.values.append((param, w))
                self.statements(self.rng.below(4), depth + 1)
                self.emit("br ^%s(%s)" % (m, self.value(w)))
            return go

        self._scoped(arm(t, tv))
        self._scoped(arm(e, ev))
        self.lines.append("^%s(%s:i%d):" % (m, mv, w))
        self.values.append((mv, w))

    def loop(self, depth):
        trips = self.const(8, self.rng.below(5))
        zero, one = self.const(8, 0), self.const(8, 1)
        init = self.value()
        w = self.width_of(init)
        head, body, out, i, acc, ii, aa, res, p = (self.fresh(x) for x in
                                                   ("head", "body", "out", "i", "acc", "ii", "aa", "res", "t"))
        self.emit("br ^%s(%s, %s)" % (head, trips, init))
        self.lines.append("^%s(%s:i8, %s:i%d):" % (head, i, acc, w))
        self.emit("%s = icmp.eq %s %s" % (p, i, zero))
        self.emit("br %s ^%s(%s) ^%s(%s, %s)" % (p, out, acc, body, i, acc))

        def go():
            self.lines.append("^%s(%s:i8, %s:i%d):" % (body, ii, aa, w))
            self.values.extend([(ii, 8), (aa, w)])
            self.statements(self.rng.below(3), depth + 1)
            nxt = self.fresh("n")
            self.emit("%s = sub %s %s" % (nxt, ii, one))
            self.emit("br ^%s(%s, %s)" % (head, nxt, self.value(w)))

        self._scoped(go)
        self.lines.append("^%s(%s:i%d):" % (out, res, w))
        self.values.append((res, w))

    def finish(self, results):
        vals = [self.value(w) for w in results]
        self.emit("ret" + "".join(" " + v for v in vals))
        return self.lines


CLASSICAL_LAYOUT = {"A_tab": 4, "H": 13, "S_cells": 0, "globals": {}, "heap_base": 16, "heap_limit_max": 4096,
                    "interface_regions": {}, "null_size": 1}
DEVICES = {"models": {"block": {"version": "0.8"}, "clock": {"version": "0.8"}, "console": {"version": "0.8"},
                      "entropy": {"version": "0.8"}}, "services": {}}


def _fn_head(kind, name, params, results):
    plist = ", ".join("arg%d:i%d" % (i, w) for i, w in enumerate(params))
    rlist = ", ".join("r%d:i%d" % (i, w) for i, w in enumerate(results))
    return "fn %s @%s (%s) (%s) () () ()" % (kind, name, plist, rlist)


def _fn_sig(kind, params, results):
    return {"abi": "0.8", "kind": kind, "params": ["i%d" % w for w in params],
            "results": ["i%d" % w for w in results], "tier": kind}


def _classical_function(rng, tier, name, params, results, helpers, ptr_width, count, extra=None):
    b = ClassicalBody(rng, tier, ptr_width, helpers)
    for i, w in enumerate(params):
        b.values.append(("arg%d" % i, w))
    if extra:
        extra(b)
    b.statements(count)
    if extra:
        extra(b)
    return [_fn_head(tier, name, params, results)] + b.finish(results)


def _arg_values(rng, widths):
    out = []
    for w in widths:
        out += ["--arg", str(rng.int_of_width(w))]
    return out


def _classical_invocations(rng, tier, params, entropy_len=64, extra_args=()):
    runs = []
    extra = list(extra_args)
    for _ in range(2):
        args = _arg_values(rng, params) + extra + ["--entropy", rng.hex_bytes(entropy_len),
                                                   "--draws", ",".join(rng.pick(DRAWS) for _ in range(8))]
        runs.append((["run-cc", "$OBJECT", "main"] + args, True))
        runs.append((["reverse", "$OBJECT", "main"] + args, True))
        runs.append((["reverse", "$OBJECT", "main", "--steps", str(rng.below(40))] + args, True))
        if tier == "cc":
            runs.append((["run-program", "$OBJECT", "main"] + args, True))
            runs.append((["run-program", "$OBJECT", "main", "--reverse"] + args, True))
    return runs


def gen_r(seed):
    rng = Prng(seed * 2 + 1)
    ptr_width = rng.pick((16, 32))
    helpers, symbols, functions = [], [], []
    for h in range(rng.below(3)):
        name = "h%d" % h
        params = [rng.pick(WIDTHS) for _ in range(rng.below(3))]
        result = rng.pick(WIDTHS)
        functions.append(_classical_function(rng, "r", name, params, [result], list(helpers), ptr_width, 1 + rng.below(6)))
        symbols.append((name, "r", _fn_sig("r", params, [result])))
        helpers.append((name, params, result))
    params = [rng.pick(WIDTHS) for _ in range(rng.below(4))]
    results = [rng.pick(WIDTHS) for _ in range(rng.below(3))]
    functions.append(_classical_function(rng, "r", "main", params, results, helpers, ptr_width, 4 + rng.below(20)))
    symbols.append(("main", "r", _fn_sig("r", params, results)))
    layout = dict(CLASSICAL_LAYOUT, ptr_width=ptr_width)
    text = _assemble(ptr_width, symbols, functions, devices=DEVICES, layout=layout)
    return Program(text, [(["check", "$OBJECT"], False)] + _classical_invocations(rng, "r", params))


# ---------------------------------------------------------------- lifted oracles

class LiftBody:
    def __init__(self, rng, ptr_width, reads, callees, regions, globals_, in_lproc, oracles=(), token=None):
        self.rng, self.ptr_width, self.callees, self.regions, self.globals = rng, ptr_width, callees, regions, globals_
        self.in_lproc, self.oracles, self.token = in_lproc, oracles, token
        self.reads = list(reads)  # (name, width) readable without declaration: inputs or params
        self.regs = []
        self.n = 0
        self.rows = []

    def reg(self, width):
        if self.rng.chance(35):
            cands = [n for n, w in self.regs if w == width]
            if cands:
                return self.rng.pick(cands)
        self.n += 1
        name = "l%d" % self.n
        self.regs.append((name, width))
        return name

    def read(self, width=None):
        cands = [n for n, w in self.reads + self.regs if width is None or w == width]
        if not cands or self.rng.chance(20):
            w = width if width is not None else self.rng.pick(WIDTHS)
            d = self.reg(w)
            self.rows.append("%s = lift.const.i%d %d" % (d, w, self.rng.int_of_width(w)))
            return d
        return self.rng.pick(cands)

    def width(self, name):
        return next(w for n, w in self.reads + self.regs if n == name)

    def address(self):
        r = self.rng
        base = None
        if (self.regions or self.globals) and r.chance(70):
            base = self.reg(self.ptr_width)
            self.rows.append("%s = lift.addr @%s" % (base, r.pick(self.regions + self.globals)))
        else:
            base = self.read(self.ptr_width)
        if r.chance(50):
            off = self.reg(self.ptr_width)
            self.rows.append("%s = lift.const.i%d %d" % (off, self.ptr_width, r.below(20)))
            p = self.reg(self.ptr_width)
            self.rows.append("%s = lift.add %s %s" % (p, base, off))
            return p
        return base

    def row(self, labels):
        r = self.rng
        k = r.below(100)
        if k < 24:
            a = self.read()
            w = self.width(a)
            b = self.read(w)
            self.rows.append("%s = lift.%s %s %s" % (self.reg(w), r.pick(BINOPS), a, b))
        elif k < 31:
            a = self.read()
            w = self.width(a)
            b = self.read(w)
            self.rows.append("%s = lift.%s %s %s" % (self.reg(w), r.pick(DIVOPS), a, b))
        elif k < 38:
            a = self.read()
            b = self.read(self.width(a))
            self.rows.append("%s = lift.icmp.%s %s %s" % (self.reg(1), r.pick(PREDS), a, b))
        elif k < 43:
            p = self.read(1)
            a = self.read()
            w = self.width(a)
            b = self.read(w)
            self.rows.append("%s = lift.select %s %s %s" % (self.reg(w), p, a, b))
        elif k < 50:
            a = self.read()
            w = self.width(a)
            t = r.pick(WIDTHS)
            if t != w:
                op = "trunc" if t < w else r.pick(("zext", "sext"))
                self.rows.append("%s = lift.%s.i%d %s" % (self.reg(t), op, t, a))
        elif k < 54:
            a = self.read()
            self.rows.append("%s = lift.mov %s" % (self.reg(self.width(a)), a))
        elif k < 58:
            a, b = self.read(), self.read()
            d1 = self.reg(self.width(a))
            d2 = self.reg(self.width(b))
            if d1 != d2:
                self.rows.append("(%s, %s) = lift.parmov (%s, %s)" % (d1, d2, a, b))
        elif k < 68:
            w = r.pick(MEM_WIDTHS)
            self.rows.append("%s = lift.load.i%d %s" % (self.reg(w), w, self.address()))
        elif k < 76:
            w = r.pick(MEM_WIDTHS)
            self.rows.append("lift.store.i%d %s %s" % (w, self.address(), self.read(w)))
        elif k < 80:
            n = r.below(16)
            self.rows.append("%s = lift.alloc %d" % (self.reg(self.ptr_width), n))
        elif k < 82:
            self.rows.append("lift.free %s" % self.read(self.ptr_width))
        elif k < 84:
            self.rows.append("lift.boundscheck %s %d" % (self.address(), r.below(9)))
        elif k < 88:
            self.rows.append("lift.faultif %s %d" % (self.read(1), r.below(16)))
        elif k < 94 and self.callees:
            name, params, result = r.pick(self.callees)
            args = [self.read(w) for w in params]
            self.rows.append("%s = lift.call @%s (%s)" % (self.reg(result), name, ", ".join(args)))
        elif k < 98 and self.oracles and self.token:
            name, in_w, tgt, flt = r.pick(self.oracles)
            ins = self.read(in_w)
            d, f = self.reg(tgt), self.reg(flt)
            if len({ins, d, f}) != 3:  # §3.3: interface operands of one call are disjoint
                return
            self.rows.append("lift.oracle_call @%s %s (%s) %s %s () ()" % (name, self.token, ins, d, f))
            if r.chance(40):
                self.rows.append("lift.oracle_uncall @%s %s (%s) %s %s () ()" % (name, self.token, ins, d, f))

    def blocks(self, count, result_width):
        r = self.rng
        lines = []
        labels = ["entry"] + ["b%d" % i for i in range(1, count)]
        label_bits = self.rng.pick((12, 16, 32))  # §3.3: 2^N above the whole index space, every symbol's blocks
        for i, label in enumerate(labels):
            lines.append("^%s():" % label)
            self.rows = []
            for _ in range(r.below(7)):
                self.row(labels)
            last = i == len(labels) - 1
            k = r.below(100)
            if last or k < 15:
                v = self.read(result_width)
                self.rows.append(("lift.ret %s" if self.in_lproc else "lift.retval %s") % v)
            elif k < 50:
                self.rows.append("qjump ^%s" % r.pick(labels[i + 1:] if r.chance(80) else labels))
            elif k < 80:
                self.rows.append("qbranch %s ^%s ^%s" % (self.read(1), r.pick(labels), r.pick(labels[i + 1:])))
            elif k < 90 and not (self.oracles and self.token):
                # §3.6/§5.5: a computed target may land on any row, so it cannot share a body with lift.oracle_uncall.
                d = self.reg(label_bits)
                self.rows.append("%s = lift.label ^%s" % (d, r.pick(labels[i + 1:])))
                self.rows.append("qjumpr %s" % d)
            else:
                self.rows.append("fault %d" % r.below(16))
            lines.extend(self.rows)
        return lines

    def regs_line(self):
        if not self.regs:
            return []
        return ["regs { %s }" % ", ".join("%s:reg[%d]" % x for x in self.regs)]


def gen_lift(seed):
    rng = Prng(seed * 2 + 1)
    ptr_width = rng.pick((16, 32))
    A_tab, H, S_cells = 4, 13, 9
    heap_base = 4112
    region_A, region_W = rng.pick(((2, 8), (4, 8), (0, 8))), None
    region_A, region_W = region_A
    use_region = rng.chance(50)
    gbytes = rng.pick((1, 4, 16))
    gname = "gdata"
    data = [(gname, rng.hex_bytes(gbytes))]
    symbols, functions = [], []
    callees = []
    for i in range(rng.below(3)):
        name = "lp%d" % i
        params = [rng.pick(WIDTHS) for _ in range(rng.below(3))]
        result = rng.pick(WIDTHS)
        visible = list(callees)
        pre = LiftBody(rng, ptr_width, [("a%d" % k, w) for k, w in enumerate(params)],
                       visible + ([(name, params, result)] if rng.chance(30) else []), [], [gname], True)
        body = pre.blocks(1 + rng.below(3), result)
        plist = ", ".join("a%d:reg[%d]" % (k, w) for k, w in enumerate(params))
        head = "lproc @%s (%s) (res:reg[%d]) () () ()" % (name, plist, result)
        functions.append([head] + pre.regs_line() + body)
        symbols.append((name, "lproc", {"abi": "0.8", "kind": "lproc", "params": params, "result": result, "tier": "q"}))
        callees.append((name, params, result))
    oracles = []
    if rng.chance(30):
        in_w = rng.pick((1, 8))
        t = rng.pick((1, 8))
        leaf = LiftBody(rng, ptr_width, [("x", in_w)], [], [], [], False)
        lb = leaf.blocks(1, t)
        functions.append(["oracle @inner (x:wire%s in) (t:wire%s) (f:wire[4]) () ()" % (_w(in_w), _w(t))] + leaf.regs_line() + lb)
        symbols.append(("inner", "oracle", {"abi": "0.8", "args": [], "fault": 4, "in": [in_w], "kind": "oracle",
                                            "mem": [], "tgt": t, "tier": "q"}))
        oracles.append(("inner", in_w, t, 4))
    in_widths = [rng.pick(WIDTHS[:8]) for _ in range(rng.below(3))]
    tgt = rng.pick(WIDTHS)
    fault = rng.pick((4, 4, 8))
    regions = ["db"] if use_region else []
    token = "tk" if oracles else None
    main = LiftBody(rng, ptr_width, [("x%d" % i, w) for i, w in enumerate(in_widths)], callees, regions, [gname],
                    False, oracles, token)
    body = main.blocks(1 + rng.below(5), tgt)
    plist = ", ".join("x%d:wire%s in" % (i, _w(w)) for i, w in enumerate(in_widths))
    mem = "db:region[%d,%d] in" % (region_A, region_W) if use_region else ""
    args = "tk:token" if token else ""
    head = "oracle @main (%s) (t:wire%s) (f:wire[%d]) (%s) (%s)" % (plist, _w(tgt), fault, mem, args)
    functions.append([head] + main.regs_line() + body)
    sig = {"abi": "0.8", "args": ["token"] if token else [], "fault": fault, "in": in_widths, "kind": "oracle",
           "mem": [{"A": region_A, "W": region_W, "role": "in"}] if use_region else [], "tgt": tgt, "tier": "q"}
    symbols.append(("main", "oracle", sig))
    symbols.append((gname, "data", {"abi": "0.8", "kind": "data", "size": gbytes, "tier": "-"}))
    layout = {"A_tab": A_tab, "H": H, "S_cells": S_cells, "globals": {gname: {"base": 16, "image": gname, "size": gbytes}},
              "heap_base": heap_base, "heap_limit_max": 2048, "interface_regions":
              {"main": {"db": {"base": 2048}}} if use_region else {}, "null_size": 16, "ptr_width": ptr_width}
    text = _assemble(ptr_width, symbols, functions, data=data, layout=layout)
    inv = [(["check", "$OBJECT"], False)]
    region_bytes = (1 << region_A) * region_W // 8
    for _ in range(3):
        inputs = ",".join(str(rng.int_of_width(w)) for w in in_widths)
        budget = str(rng.pick((0, 1, 2, 3, 5, 8, 13, 21, 64, 256)))
        extra = ["--mem", "db=" + rng.hex_bytes(region_bytes)] if use_region else []
        if token:
            extra += ["--arg", "token:%d" % rng.pick((0, 1, 16, 64))]
        inv.append((["run-oracle", "$OBJECT", "main", "--input", inputs, "--budget", budget] + extra, False))
        inv.append((["reverse", "$OBJECT", "main", "--input", inputs, "--budget", budget] + extra, False))
        inv.append((["reverse", "$OBJECT", "main", "--input", inputs, "--budget", budget, "--steps",
                     str(rng.below(int(budget) + 1))] + extra, False))
    tok = ["--arg", "token:16"] if token else []
    inv.append((["step-check", "$OBJECT", "main", "--samples", "4", "--seed", "%d,%d,%d,%d" % tuple(
        rng.below(1 << 30) + 1 for _ in range(4)), "--budget", "32"] + tok, False))
    return Program(text, inv)


def _w(n):
    return "" if n == 1 else "[%d]" % n


# ---------------------------------------------------------------- structured actions

class ActionBody:
    """Builds a linear action body; `perm` restricts it to permutation gates (§5.3)."""

    def __init__(self, rng, perm, mut, ins, subs, oracles, token, ptrw):
        self.rng, self.perm, self.mut, self.ins = rng, perm, mut, ins
        self.subs, self.oracles, self.token = subs, oracles, token
        self.lines = ["^entry():"]
        self.bounds = []
        self.n = 0
        self.region = None

    def wires(self, table):
        out = []
        for name, w in table:
            out.extend([name] if w == 1 else ["%s[%d]" % (name, i) for i in range(w)])
        return out

    def fresh(self, stem):
        self.n += 1
        return "%s%d" % (stem, self.n)

    def statements(self, count, blocked, depth, muts=None):
        """blocked: wires under an effective control or selector, never targets. Emits at least one line."""
        before = len(self.lines)
        for _ in range(count):
            self.statement(blocked, depth)
        if len(self.lines) == before:
            self.lines.append("x %s" % self._targets(blocked)[0])

    def _targets(self, blocked):
        return [w for w in self.wires(self.mut) if not _hits(w, blocked)]

    def _controls(self, exclude):
        return [w for w in self.wires(self.mut) + self.wires(self.ins) if not _hits(w, exclude)]

    def statement(self, blocked, depth):
        r = self.rng
        k = r.below(100)
        targets = self._targets(blocked)
        if not targets:
            return
        t = r.pick(targets)
        if k < 22:
            self.lines.append("x %s" % t)
        elif k < 40:
            if self.perm:
                self.lines.append("x %s" % t)
            else:
                g = r.pick(("h", "s", "sdg", "t", "tdg"))
                self.lines.append("%s %s" % (g, t))
        elif k < 46 and not self.perm:
            self.lines.append("%s %s %s" % (r.pick(("rx", "ry", "rz")), repr(r.pick((0.5, 1.0, -0.25, 3.141592653589793, 1e-3))), t))
        elif k < 58:
            c = self._controls(set(blocked) | {t})
            if c:
                self.lines.append("cx %s %s" % (r.pick(c), t))
        elif k < 63:
            c = self._controls(set(blocked) | {t})
            if len(c) >= 2:
                a = r.pick(c)
                b = r.pick([x for x in c if x != a])
                self.lines.append("ccx %s %s %s" % (a, b, t))
        elif k < 66:
            c = [x for x in self._controls(set(blocked) | {t}) if "[" not in x]  # §16.2: lists hold bare NAMEs
            if len(c) >= 2:
                a = r.pick(c)
                b = r.pick([x for x in c if x != a])
                self.lines.append("mcx (%s, %s) %s" % (a, b, t))
        elif k < 70:
            others = [x for x in targets if x != t]
            if others:
                self.lines.append("swap %s %s" % (t, r.pick(others)))
        elif k < 73 and not self.perm:
            others = [x for x in self.wires(self.mut) + self.wires(self.ins) if x != t and not _hits(x, blocked)]
            if others:
                self.lines.append("cz %s %s" % (t, r.pick(others)))
        elif k < 80 and depth < 2:
            c = [x for x in self._controls(blocked) if self._targets(set(blocked) | {x})]
            if c:
                sel = r.pick(c)
                self.lines.append("qif %s {" % sel)
                self.statements(1 + r.below(2), set(blocked) | {sel}, depth + 1)
                self.lines.append("} {")
                self.statements(1 + r.below(2), set(blocked) | {sel}, depth + 1)
                self.lines.append("}")
        elif k < 85 and depth < 2:
            n = 1 + r.below(3)
            self.bounds.append(n)
            self.lines.append("qloop %d {" % n)
            self.statements(1 + r.below(2), blocked, depth + 1)
            self.lines.append("}")
        elif k < 88 and depth < 2:
            bundles = [(nm, w) for nm, w in self.ins if w >= 2 and not any(_hits(x, blocked) for x in self._bits(nm, w))]
            if bundles:  # `in` bundles are never targets, so the body's targets are unaffected
                nm, w = r.pick(bundles)
                self.bounds.append(w)
                self.lines.append("qloop %d while %s {" % (w, nm))
                self.statements(1 + r.below(2), set(blocked) | set(self._bits(nm, w)), depth + 1)
                self.lines.append("}")
        elif k < 91:
            c = [x for x in self._controls(set(blocked) | {t}) if self._targets(set(blocked) | {x})]
            if c:
                sel = r.pick(c)
                self.lines.append("qpred %s" % sel)
                self.statements(1, set(blocked) | {sel}, depth + 1)
                self.lines.append("qunpred %s" % sel)
        elif k < 93:
            self.copy_window(blocked)
        elif k < 95:
            self.local_workspace(blocked)
        elif k < 97:
            self.region_window(blocked)
        elif self.subs:
            self.sub_call(blocked)

    def _bits(self, name, w):
        return [name] if w == 1 else ["%s[%d]" % (name, i) for i in range(w)]

    def copy_window(self, blocked):
        """qcopy s; use; quncopy s (§6.2): the source is not modified inside the window."""
        r = self.rng
        cands = [(n, w) for n, w in self.ins]
        same = [(s, t) for s in cands for t in self.mut if t[1] == s[1] and not any(_hits(x, blocked) for x in self._bits(t[0], t[1]))]
        if not same:
            return
        (src, w), (tgt, _) = r.pick(same)
        d = self.fresh("d")
        self.lines.append("%s = qcopy %s" % (d, src))
        self.lines.append("qxor %s %s" % (d, tgt))
        self.lines.append("quncopy %s %s" % (src, d))

    def local_workspace(self, blocked):
        """qalloc; compute; exact adjoint of the computation; qfree (§4.3, §5.1)."""
        r = self.rng
        w = self.fresh("w")
        c = self._controls(blocked)
        self.lines.append("%s = qalloc any 1" % w)
        if c and r.chance(60):
            ctl = r.pick(c)
            self.lines.append("cx %s %s" % (ctl, w))
            self.lines.append("cx %s %s" % (ctl, w))
        elif not self.perm:
            self.lines.append("h %s" % w)
            self.lines.append("h %s" % w)
        else:
            self.lines.append("x %s" % w)
            self.lines.append("x %s" % w)
        self.lines.append("qfree %s" % w)

    def region_window(self, blocked):
        """A local region (§4.6): qload/qunload and self-inverse qstore/qexchm pairs restore its image."""
        r = self.rng
        if self.region is None:
            return
        name, a_bits, cell = self.region
        addrs = [n for n, w in self.ins if w == a_bits]
        data = [n for n, w in self.mut if w == cell and not any(_hits(x, blocked) for x in self._bits(n, w))]
        if not addrs:
            return
        addr = r.pick(addrs)
        k = r.below(3)
        if k == 0 or not data:
            d = self.fresh("d")
            self.lines.append("%s = qload %s %s" % (d, name, addr))
            self.lines.append("qunload %s %s %s" % (name, addr, d))
        else:
            v = r.pick(data)
            op = "qstore" if k == 1 else "qexchm"
            self.lines.append("%s %s %s %s" % (op, name, addr, v))
            self.lines.append("%s %s %s %s" % (op, name, addr, v))

    def sub_call(self, blocked):
        r = self.rng
        name, width, adjointable = r.pick(self.subs)
        cands = [(n, w) for n, w in self.mut if w == width and not any(_hits(x, blocked) for x in self._bits(n, w))]
        if not cands:
            return
        target = r.pick(cands)[0]
        prefix = "adj " if r.chance(30) else ""
        ctl = ""
        if r.chance(40):
            c = [x for x in self._controls(blocked) if not x.startswith(target + "[") and x != target]
            if c:
                ctl = "ctl %s " % r.pick(c)
        self.lines.append("%s%saction_call @%s (%s) () () ()" % (ctl, prefix, name, target))

    def indent_free(self):
        return self.lines


def _hits(wire, blocked):
    if wire in blocked:
        return True
    base = wire.split("[")[0]
    return base in blocked or any(b.split("[")[0] == wire for b in blocked)


def _action_function(rng, name, perm, mut, ins, subs, count, image=None):
    b = ActionBody(rng, perm, mut, ins, subs, [], None, 16)
    if image is not None:
        b.lines.append("qregion r %d %d @%s" % (image[1], image[2], image[0]))
        b.region = ("r", image[1], image[2])
    b.statements(count, set(), 0)
    if len(b.lines) == 1:
        b.lines.append("x %s" % b.wires(mut)[0])
    kind = "perm_action" if perm else "action"
    plist = ", ".join("%s:wire%s mut" % (n, _w(w)) for n, w in mut)
    ilist = ", ".join("%s:wire%s in" % (n, _w(w)) for n, w in ins)
    head = "%s @%s (%s) (%s) () () ()" % (kind, name, plist, ilist)
    regs = ["regs {  } bounds {%s}" % ", ".join(str(x) for x in b.bounds)] if b.bounds else []
    sig = {"abi": "0.8", "args": [], "bounds": b.bounds, "in": [w for _, w in ins], "kind": kind, "mem": [],
           "mut": [w for _, w in mut], "tier": "q"}
    return [head] + regs + b.lines, (name, kind, sig)


def _state_arg(rng, n):
    """A small dyadic superposition over distinct basis strings, in the --state spelling."""
    picks = sorted({"".join(rng.pick("01") for _ in range(n)) for _ in range(2)})
    amps = ["0.5", "0,0.5", "-0.5", "0.25,0.25"]
    return ";".join("%s=%s" % (p, rng.pick(amps)) for p in picks)


def gen_q(seed):
    rng = Prng(seed * 2 + 1)
    symbols, functions, subs = [], [], []
    for i in range(rng.below(3)):
        w = rng.pick((1, 2))
        perm = rng.chance(50)
        fn, sym = _action_function(rng, "s%d" % i, perm, [("t", w)], [], list(subs) if perm else [s for s in subs], 1 + rng.below(3))
        functions.append(fn)
        symbols.append(sym)
        subs.append(("s%d" % i, w, True))
    mut = [("t%d" % i, rng.pick((1, 2, 3))) for i in range(1 + rng.below(2))]
    ins = [("c%d" % i, rng.pick((1, 2))) for i in range(rng.below(3))]
    perm = rng.chance(30)
    perm_subs = [s for s in subs if next(y for y in symbols if y[0] == s[0])[1] == "perm_action"]
    image, data = None, ()
    if rng.chance(40):
        a_bits, cell = rng.pick(((1, 8), (2, 8), (1, 16)))
        ins.append(("addr", a_bits))
        size = (1 << a_bits) * cell // 8
        image = ("img", a_bits, cell)
        data = (("img", rng.hex_bytes(size)),)
        symbols.append(("img", "data", {"abi": "0.8", "kind": "data", "size": size, "tier": "-"}))
        if not any(w == cell for _, w in mut):
            mut.append(("cellv", cell))
    fn, sym = _action_function(rng, "main", perm, mut, ins, perm_subs if perm else subs, 2 + rng.below(9), image)
    functions.append(fn)
    symbols.append(sym)
    text = _assemble(32, symbols, functions, data=data)
    n = sum(w for _, w in mut) + sum(w for _, w in ins)
    inv = [(["check", "$OBJECT"], False)]
    for _ in range(2):
        basis = "".join(rng.pick("01") for _ in range(n))
        for cmd in ("run-action", "reverse"):
            inv.append(([cmd, "$OBJECT", "main", "--basis", basis], False))
            inv.append(([cmd, "$OBJECT", "main", "--basis", basis, "--adjoint"], False))
        state = _state_arg(rng, n)
        inv.append((["run-action", "$OBJECT", "main", "--state", state], False))
        inv.append((["reverse", "$OBJECT", "main", "--state", state], False))
    inv.append((["run-action", "$OBJECT", "main", "--mut", ",".join(str(rng.int_of_width(w)) for _, w in mut)]
                + (["--in", ",".join(str(rng.int_of_width(w)) for _, w in ins)] if ins else []), False))
    return Program(text, inv)


# ---------------------------------------------------------------- CC with registers, measurement, region calls

def gen_cc(seed):
    rng = Prng(seed * 2 + 1)
    symbols, functions = [], []
    width = rng.pick((1, 2, 3))
    # a permutation action, a superposing action and a memoryless oracle reachable from CC.
    fn, sym = _action_function(rng, "perm", True, [("t", width)], [], [], 1 + rng.below(3))
    functions.append(fn)
    symbols.append(sym)
    fn, sym = _action_function(rng, "mix", False, [("t", width)], [], [], 1 + rng.below(3))
    functions.append(fn)
    symbols.append(sym)
    ptr_width = rng.pick((16, 32))
    leaf = LiftBody(rng, ptr_width, [("x", width)], [], [], [], False)
    body = leaf.blocks(1 + rng.below(2), width)
    functions.append(["oracle @orc (x:wire%s in) (t:wire%s) (f:wire[4]) () ()" % (_w(width), _w(width))]
                     + leaf.regs_line() + body)
    symbols.append(("orc", "oracle", {"abi": "0.8", "args": [], "fault": 4, "in": [width], "kind": "oracle",
                                      "mem": [], "tgt": width, "tier": "q"}))
    params = [rng.pick(WIDTHS) for _ in range(rng.below(3))]
    helpers = []
    for h in range(rng.below(2)):
        name = "h%d" % h
        hp = [rng.pick(WIDTHS) for _ in range(rng.below(2))]
        result = rng.pick(WIDTHS)
        functions.append(_classical_function(rng, "cc", name, hp, [result], [], ptr_width, 1 + rng.below(4)))
        symbols.append((name, "cc", _fn_sig("cc", hp, [result])))
        helpers.append((name, hp, result))
    results = [width]
    b = ClassicalBody(rng, "cc", ptr_width, helpers)
    for i, w in enumerate(params):
        b.values.append(("arg%d" % i, w))
    b.statements(rng.below(5))
    q = "qmain"
    b.emit("%s = qreg.new any %d" % (q, width))
    live = [q]
    measured = None
    for i in range(1 + rng.below(4)):
        k = rng.below(5)
        if k == 0:
            b.emit("action_call @perm (%s) () () ()" % q)
        elif k == 1:
            b.emit("adj action_call @mix (%s) () () ()" % q if rng.chance(30) else "action_call @mix (%s) () () ()" % q)
        elif k == 2:
            d = "qo%d" % i
            f = "qf%d" % i
            b.emit("%s = qreg.new any %d" % (d, width))
            b.emit("%s = qreg.new any 4" % f)
            b.emit("oracle_call @orc tk (%s) %s %s () ()" % (q, d, f))
            live += [d, f]
        elif k == 3 and width >= 2:
            s = "qs%d" % i
            b.emit("%s = qreg.slice %s 0 %d" % (s, q, width))
            b.emit("action_call @perm (%s) () () ()" % s)
        else:
            a, c = "qa%d" % i, "qc%d" % i
            e = "qe%d" % i
            b.emit("%s = qreg.new any 1" % a)
            b.emit("%s = qreg.new any 1" % c)
            b.emit("%s = qreg.concat %s %s" % (e, a, c))
            live.append(e)
        b.statements(rng.below(2))
    measured = "qm0"
    b.emit("%s = measure %s" % (measured, q))
    b.values.append((measured, width))
    for extra in live[1:]:
        if rng.chance(50):
            b.emit("x%s = measure %s" % (extra, extra))
        else:
            b.emit("qreg.free %s" % extra)
    b.emit("ret %s" % measured)
    plist = ", ".join("arg%d:i%d" % (i, w) for i, w in enumerate(params))
    plist = plist + ", " if plist else plist
    head = "fn cc @main (%stk:token) (r0:i%d) () () ()" % (plist, width)
    functions.append([head] + b.lines)
    sig = {"abi": "0.8", "kind": "cc", "params": ["i%d" % w for w in params] + ["token"],
           "results": ["i%d" % width], "tier": "cc"}
    symbols.append(("main", "cc", sig))
    layout = dict(CLASSICAL_LAYOUT, ptr_width=ptr_width)
    text = _assemble(ptr_width, symbols, functions, devices={"models": {}, "services": {}}, layout=layout)
    token = ["--arg", "token:%d" % rng.pick((0, 1, 8, 64))]
    return Program(text, [(["check", "$OBJECT"], False)] + _classical_invocations(rng, "cc", params, extra_args=token))


def gen_mutant(seed):
    """A well-formed program with one random edit and recomputed identity. It may or may not stay valid:
    what is checked is that executors agree on accepting it and on its behaviour, finding validator gaps."""
    rng = Prng(seed * 2 + 1)
    base = GENERATORS[rng.pick(("r", "lift", "cc", "q"))](rng.below(1 << 30))
    o = objfile.parse(base.text)
    f = rng.pick(o.functions)
    i = 1 + rng.below(len(f.lines) - 1)
    k = rng.below(4)
    if k == 0:
        del f.lines[i]
    elif k == 1:
        f.lines.insert(i, f.lines[i])
    elif k == 2:
        j = 1 + rng.below(len(f.lines) - 1)
        f.lines[i], f.lines[j] = f.lines[j], f.lines[i]
    else:
        f.lines[i] = _mutate_tokens(rng, f.lines[i])
    try:
        text = identity.rehash(o).text()
    except (identity.IdentityError, KeyError, ValueError, objfile.ObjectError):
        text = base.text  # the edit broke references beyond identity: keep the program, it is still driven
    return Program(text, base.invocations)


def _mutate_tokens(rng, line):
    toks = line.split(" ")
    i = rng.below(len(toks))
    tok = toks[i]
    if tok.isdigit():
        toks[i] = str(rng.int_of_width(8))
    elif ".i" in tok:
        toks[i] = tok.split(".i")[0] + ".i%d" % rng.pick(WIDTHS)
    elif rng.chance(50) and len(toks) > 1:
        toks[i] = toks[rng.below(len(toks))]
    else:
        toks[i] = tok + "x"
    return " ".join(toks)


GENERATORS = {"r": gen_r, "lift": gen_lift, "cc": gen_cc, "q": gen_q, "mutant": gen_mutant}


def program(cls, seed):
    if cls not in GENERATORS:
        raise ValueError("unknown generator class %r (known: %s)" % (cls, ", ".join(CLASSES)))
    return GENERATORS[cls](seed)


def cases(cls, seed):
    p = program(cls, seed)
    corpus = "gen-" + cls
    out = []
    for i, (argv, tape) in enumerate(p.invocations):
        out.append(engine.Case(corpus, "seed%d/%d" % (seed, i), argv, object_text=p.text, tape=tape))
    return out


def run_class(runner, cls, first_seed, wall_seconds, count, jobs):
    """Run seeds first_seed, first_seed+1, ... until the wall bound or count; the seed range is reported."""
    deadline = time.monotonic() + wall_seconds
    seed = first_seed
    programs = rejected = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
        while (count and programs < count) or (not count and time.monotonic() < deadline):
            batch_seeds = list(range(seed, seed + (min(jobs * 2, count - programs) if count else jobs * 2)))
            batch = [c for s in batch_seeds for c in cases(cls, s)]
            for case, outcomes in zip(batch, pool.map(runner.execute, batch)):
                runner.account(runner.result_of(case, outcomes))
                if cls in WELL_FORMED and case.argv[0] == "check" and all(o.kind != "json" for o in outcomes.values()):
                    rejected += 1
                    runner.note_generator_rejection(cls, case, outcomes)
            programs += len(batch_seeds)
            seed += len(batch_seeds)
    return {"programs": programs, "seeds": [first_seed, seed - 1], "wall_bound_seconds": wall_seconds,
            "rejected_by_every_executor": rejected}
