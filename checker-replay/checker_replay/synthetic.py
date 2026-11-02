"""Objects for row families the published corpus lacks.

Each case states its relation in `facts` as a hand-derived function, not the replay's
output, so a disagreement cannot be manufactured by the replay agreeing with itself.
Certificates carry the full §12.3 region schema; values outside the replay's scope
are minimal so a checker that rejects them is seen at document level, not claim level.
"""
import itertools
from . import hashes as H, jcs
from .objparse import parse_object, type_width, region_shape

HEAD = "bennett-object isa=0.9 object=0.9 launch=0.9 sched=0.9 cert=0.9 coverage=0.9 target=0.9 ptr_width=8\n"
VERSIONS = {k: "0.9" for k in ("cert", "coverage", "isa", "launch", "object", "sched", "target")}


def build_object(fns, layout=None, data=None):
    text = HEAD + "\nsection symbols\n{}\n\nsection functions\n" + fns
    if data:
        text += "\nsection data\n" + data
    if layout:
        text += "\nsection layout\n" + jcs.canonical(layout).decode() + "\n"
    obj = parse_object(text)

    def mem(group):
        return [dict(zip("AW", region_shape(t)), role=role) for _n, t, role in group]
    syms = {}
    for i, (name, fn) in enumerate(obj.functions.items()):
        g = fn.groups
        w = lambda grp: [type_width(t, obj.ptr_width) for _n, t, _s in grp]
        if fn.kind == 'oracle':
            sig = {"abi": "0.9", "args": [t for _n, t, _s in g[4]], "fault": w(g[2])[0], "in": w(g[0]), "kind": "oracle",
                   "mem": mem(g[3]), "tgt": w(g[1])[0], "tier": "q"}
        elif fn.kind == 'lproc':
            sig = {"abi": "0.9", "args": [t for _n, t, _s in g[3]], "kind": "lproc", "mem": mem(g[4]), "params": w(g[0]),
                   "result": w(g[1])[0], "tier": "q", "tokens": [t for _n, t, _s in g[2]]}
        else:
            sig = {"abi": "0.9", "args": [t for _n, t, _s in g[3]], "bounds": fn.bounds, "in": w(g[1]), "kind": fn.kind,
                   "mem": mem(g[2]), "mut": w(g[0]), "tier": "q"}
        syms[str(i)] = {"interface_hash": "0" * 64, "kind": fn.kind, "name": name, "signature": sig}
    n = len(syms)
    for dname, hx in sorted((obj.data or {}).items()):
        syms[str(n)] = {"interface_hash": "0" * 64, "kind": "data", "name": dname,
                        "signature": {"abi": "0.9", "kind": "data", "size": len(hx) // 2, "tier": "-"}}
        n += 1

    def render(s):
        return text.replace("section symbols\n{}", "section symbols\n" + jcs.canonical(s).decode())
    h = H.compute_hashes(parse_object(render(syms)))
    for s in syms.values():
        s["interface_hash"] = h[s["name"]][0]
    return render(syms)


def _frame(fn):
    g = fn.groups
    slots = []
    if fn.kind == 'oracle':
        for role, grp in (("in", g[0]), ("tgt", g[1]), ("fault", g[2])):
            for i, (_n, t, _s) in enumerate(grp):
                slots.append({"class": "fast", "id": f"{role}:{i}", "role": role, "width": type_width(t, 8)})
    else:
        for role, grp in (("mut", g[0]), ("in", g[1])):
            for i, (_n, t, _s) in enumerate(grp):
                slots.append({"class": "fast", "id": f"{role}:{i}", "role": role, "width": type_width(t, 8)})
    return {"ancillae": [], "borrow": [], "history": [], "scratch": [], "slots": slots}


def certificate(text, root, claim_builder, verification=None):
    obj = parse_object(text)
    hs = H.compute_hashes(obj)
    ch = hs[root][1]
    sym = obj.symbol_by_name[root]
    claims = claim_builder(ch, hs)
    ver = {"borrow_frames": [], "channel": [], "fault_rate": {"domain": {"borrowed": [], "bundles": [], "regions": []},
                                                              "faults_by_code": {}, "layout_dependent_excluded": 0,
                                                              "undefined_excluded": 0},
           "inverse_pairs": [], "oracle": None, "unitary": []}
    if verification:
        ver.update(verification(hs))
    rec = {"bounds": sym["signature"].get("bounds", []), "claims": claims, "cleanup": {}, "definedness_assumptions": [],
           "error": {}, "frame": _frame(obj.functions[root]), "gates_used": {}, "kind": sym["kind"], "resources": {},
           "signature": sym["signature"], "verification": ver}
    return {"bindings": {"closure_hash": ch, "dialect_mapping_version": "none", "interface_hash": hs[root][0],
                         "object_hash": H.object_hash(text), "reference_program_hash": hs[root][0], "target_hash": "0" * 64},
            "composition": {"epsilon_total": {"diamond": 0}, "method": "sum_over_multiplicities", "multiplicities": {}},
            "producer": {"materialisation": {"llvm_version": "none", "normaliser_version": "none", "pipeline": "synthetic"},
                         "name": "checker-replay", "version": "1"},
            "regions": {ch: rec}, "versions": dict(VERSIONS)}


def rng(name, width, lo=0, hi=None):
    return {"name": name, "width": width, "lo": lo, "hi": (1 << width) - 1 if hi is None else hi}


def tup(b, bo=(), r=()):
    return {"bundles": [str(x) for x in b], "borrowed": [str(x) for x in bo], "regions": [[str(c) for c in x] for x in r]}


def oracle_case(name, fns, in_widths, tgt, fault, budget, fact, args=(), layout=None, data=None, root="o", inames=None,
                label="evidence", regions=None):
    """Claim over ins with target/fault zero; `fact(ins) -> (payload, code)` is the hand relation."""
    text = build_object(fns, layout, data)
    inames = inames or [f"x{i}" for i in range(len(in_widths))]

    def build(ch, hs):
        dom = [rng(n, w) for n, w in zip(inames, in_widths)]
        rows = []
        for ins in itertools.product(*[range(1 << w) for w in in_widths]):
            pay, code = fact(*ins)
            rows.append({"before": tup(list(ins) + [0, 0], (), regions or ()),
                         "after": tup(list(ins) + [pay, code], (), regions or ())})
        reg_dom = []
        if regions:
            sym = parse_object(text).functions[root].groups[3]
            for (n, _t, _s), cells in zip(sym, regions):
                reg_dom.append({"name": n, "cells": [{"index": i, "lo": c, "hi": c} for i, c in enumerate(cells)]})
        return [{"kind": "exhaustive", "label": label, "subject": {"closure_hash": ch, "variant": "forward", "inherited_controls": 0},
                 "invocation": {"args": list(args), "budget": [str(x) for x in budget]},
                 "domain": {"bundles": dom, "borrowed": [], "regions": reg_dom}, "expected": {"table": rows}}]
    return name, text, certificate(text, root, build)


def action_case(name, fns, widths, fact, inherited=0, root="a", args=(), budget=(0, 0, 0, 0), names=None, borrowed=(), layout=None, data=None,
                fix_regions=None):
    text = build_object(fns, layout, data)
    g = parse_object(text).functions[root].groups
    names = names or [n for n, _t, _s in list(g[0]) + list(g[1])]
    prefix = [rng(f"c{i}", 1) for i in range(inherited)]
    ctl = [1] * inherited

    def build(ch, hs):
        rows = []
        space = ctl + list(widths) + list(borrowed)
        for vals in itertools.product(*[range(1 << w) for w in space]):
            b = vals[:inherited + len(widths)]
            bo = vals[inherited + len(widths):]
            rows.append({"before": tup(b, bo), "after": tup(fact(*vals)[:len(b)], fact(*vals)[len(b):])})
        return [{"kind": "exhaustive", "label": "proof", "subject": {"closure_hash": ch, "variant": "forward", "inherited_controls": inherited},
                 "invocation": {"args": list(args), "budget": [str(x) for x in budget]},
                 "domain": {"bundles": prefix + [rng(n, w) for n, w in zip(names, widths)],
                            "borrowed": [rng(f"b{i}", w) for i, w in enumerate(borrowed)], "regions": []},
                 "expected": {"table": rows}}]
    return name, text, certificate(text, root, build)


O2 = "oracle @o (x0:wire[8] in, x1:wire[8] in) (t:wire[8]) (f:wire[4]) () ()\n"
O1 = "oracle @o (x0:wire[4] in) (t:wire[4]) (f:wire[4]) () ()\n"


def _sg(v, w=4):
    return v - (1 << w) if v >> (w - 1) else v


def _o1(tag, body, regs, fact, budget=(20, 0, 0, 0), **kw):
    return oracle_case(f"oracle-{tag}", O1 + f"regs {{ {regs} }}\n^entry():\n{body}", [4], 4, 4, budget, fact, **kw)


def _shift_case(op, fn):
    src = ("oracle @o (x0:wire[4] in, x1:wire[4] in) (t:wire[4]) (f:wire[4]) () ()\n"
           f"regs {{ r:reg[4] }}\n^entry():\nr = {op} x0 x1\nlift.retval r\n")
    return oracle_case(f"oracle-{op}", src, [4, 4], 4, 4, (2, 0, 0, 0), fn)


LAYOUT = {"A_tab": 2, "H": 6, "S_cells": 5, "globals": {"g": {"base": 4, "image": "g", "size": 2}}, "heap_base": 16,
          "heap_limit_max": 32, "interface_regions": {}, "null_size": 4, "ptr_width": 8}


def _sdiv(a, b):
    if b == 0:
        return 0, 5
    sa, sb = _sg(a), _sg(b)
    if a == 8 and b == 15:
        return 0, 6
    q = abs(sa) // abs(sb)
    q = -q if (sa < 0) != (sb < 0) else q
    return q & 15, 0


def _srem(a, b):
    if b == 0:
        return 0, 5
    if a == 8 and b == 15:
        return 0, 6
    sa, sb = _sg(a), _sg(b)
    q = abs(sa) // abs(sb)
    q = -q if (sa < 0) != (sb < 0) else q
    return (sa - q * sb) & 15, 0


def synthetic_cases():
    out = [
        _shift_case("lift.shl", lambda a, b: ((a << b) & 15 if b < 4 else 0, 0)),
        _shift_case("lift.lshr", lambda a, b: (a >> b if b < 4 else 0, 0)),
        _shift_case("lift.ashr", lambda a, b: ((_sg(a) >> b) & 15 if b < 4 else 0, 0)),
        _shift_case("lift.mul", lambda a, b: ((a * b) & 15, 0)),
        _shift_case("lift.udiv", lambda a, b: (a // b, 0) if b else (0, 5)),
        _shift_case("lift.urem", lambda a, b: (a % b, 0) if b else (0, 5)),
        _shift_case("lift.sdiv", _sdiv),
        _shift_case("lift.srem", _srem),
    ]
    for pred, fn in (("eq", lambda a, b: a == b), ("ne", lambda a, b: a != b), ("ult", lambda a, b: a < b),
                     ("ule", lambda a, b: a <= b), ("slt", lambda a, b: _sg(a) < _sg(b)), ("sle", lambda a, b: _sg(a) <= _sg(b))):
        src = ("oracle @o (x0:wire[4] in, x1:wire[4] in) (t:wire[1]) (f:wire[4]) () ()\n"
               f"regs {{ r:reg[1] }}\n^entry():\nr = lift.icmp.{pred} x0 x1\nlift.retval r\n")
        out.append(oracle_case(f"oracle-icmp-{pred}", src, [4, 4], 1, 4, (2, 0, 0, 0), lambda a, b, fn=fn: (int(fn(a, b)), 0)))
    out += [
        _o1("sext-trunc", "w = lift.sext.i8 x0\nr = lift.trunc.i4 w\nlift.retval r\n", "w:reg[8], r:reg[4]", lambda x: (x, 0)),
        _o1("sext-high", "w = lift.sext.i8 x0\ns = lift.const.i8 4\nq = lift.lshr w s\nr = lift.trunc.i4 q\nlift.retval r\n",
            "w:reg[8], s:reg[8], q:reg[8], r:reg[4]", lambda x: (15 if x >> 3 else 0, 0)),
        _o1("const-negative-spelling", "c = lift.const.i4 -1\nr = lift.xor x0 c\nlift.retval r\n", "c:reg[4], r:reg[4]", lambda x: (x ^ 15, 0)),
        _o1("parmov", "a = lift.mov x0\nb = lift.const.i4 3\n(a, b) = lift.parmov (b, a)\nr = lift.sub a b\nlift.retval r\n",
            "a:reg[4], b:reg[4], r:reg[4]", lambda x: ((3 - x) & 15, 0)),
        _o1("branch", "z = lift.const.i4 0\np = lift.icmp.eq x0 z\nqbranch p ^y ^n\n^y():\nlift.retval x0\n^n():\nfault 9\n",
            "z:reg[4], p:reg[1]", lambda x: (0, 0) if x == 0 else (0, 9)),
        _o1("faultif-zero-literal", "z = lift.const.i4 0\np = lift.icmp.ne x0 z\nlift.faultif p 0\nlift.retval x0\n",
            "z:reg[4], p:reg[1]", lambda x: (0, 0) if x == 0 else (0, 4)),
        _o1("budget-exhaust-loop", "qjump ^entry\n", "r:reg[4]", lambda x: (0, 1), budget=(7, 0, 0, 0)),
        _o1("retval-on-last-step", "r = lift.mov x0\nlift.retval r\n", "r:reg[4]", lambda x: (x, 0), budget=(2, 0, 0, 0)),
        _o1("budget-one-short", "r = lift.mov x0\nlift.retval r\n", "r:reg[4]", lambda x: (0, 1), budget=(1, 0, 0, 0)),
        _o1("budget-zero", "r = lift.mov x0\nlift.retval r\n", "r:reg[4]", lambda x: (0, 1), budget=(0, 0, 0, 0)),
        _o1("label-index", "w = lift.label ^b\nqjump ^b\n^c():\nfault 9\n^b():\nlift.retval w\n", "w:reg[4]", lambda x: (3, 0)),
        _o1("qjumpr-in-range", "w = lift.label ^b\nqjumpr w\n^c():\nfault 9\n^b():\nlift.retval w\n", "w:reg[4]", lambda x: (3, 0)),
        _o1("qjumpr-computed", "qjumpr x0\n^a():\nlift.retval x0\n^b():\nfault 9\n", "r:reg[4]",
            lambda x: (0, 4) if x > 2 else ((0, 9) if x == 2 else ((1, 0) if x == 1 else (0, 1)))),
    ]
    sym_layout = dict(LAYOUT)
    mem_hdr = O1
    out += [
        oracle_case("oracle-global-load", mem_hdr + "regs { p:reg[8], v:reg[8], r:reg[4] }\n^entry():\np = lift.addr @g\nv = lift.load.i8 p\nr = lift.trunc.i4 v\nlift.retval r\n",
                    [4], 4, 4, (9, 0, 0, 0), lambda x: (0xa, 0), layout=sym_layout, data="g 0a0b\n"),
        oracle_case("oracle-load-past-end", mem_hdr + "regs { p:reg[8], o:reg[8], q:reg[8], v:reg[16], r:reg[4] }\n^entry():\np = lift.addr @g\no = lift.zext.i8 x0\nq = lift.add p o\nv = lift.load.i16 q\nr = lift.trunc.i4 v\nlift.retval r\n",
                    [4], 4, 4, (9, 0, 0, 0), lambda x: (0xa, 0) if x == 0 else (0, 7), layout=sym_layout, data="g 0a0b\n"),
        oracle_case("oracle-store-load-heap", mem_hdr + "regs { n:reg[8], p:reg[8], v:reg[8], w:reg[8], r:reg[4] }\n^entry():\nn = lift.const.i8 1\np = lift.alloc n\nw = lift.zext.i8 x0\nlift.store.i8 p w\nv = lift.load.i8 p\nr = lift.trunc.i4 v\nlift.retval r\n",
                    [4], 4, 4, (9, 4, 0, 0), lambda x: (x, 0), layout=sym_layout, data="g 0a0b\n"),
        oracle_case("oracle-heap-limit", mem_hdr + "regs { n:reg[8], p:reg[8], r:reg[4] }\n^entry():\nn = lift.zext.i8 x0\np = lift.alloc n\nr = lift.trunc.i4 p\nlift.retval r\n",
                    [4], 4, 4, (9, 4, 0, 0), lambda x: (0, 0) if x <= 4 else (0, 2), layout=sym_layout, data="g 0a0b\n"),
        oracle_case("oracle-alloc-zero-bumps", mem_hdr + "regs { n:reg[8], p:reg[8], q:reg[8], d:reg[8], r:reg[4] }\n^entry():\nn = lift.const.i8 0\np = lift.alloc n\nq = lift.alloc n\nd = lift.sub q p\nr = lift.trunc.i4 d\nlift.retval r\n",
                    [4], 4, 4, (9, 4, 0, 0), lambda x: (1, 0), layout=sym_layout, data="g 0a0b\n"),
        oracle_case("oracle-double-free", mem_hdr + "regs { n:reg[8], p:reg[8] }\n^entry():\nn = lift.const.i8 1\np = lift.alloc n\nlift.free p\nlift.free p\nlift.retval x0\n",
                    [4], 4, 4, (9, 4, 0, 0), lambda x: (0, 7), layout=sym_layout, data="g 0a0b\n"),
        oracle_case("oracle-free-pinned", mem_hdr + "regs { p:reg[8] }\n^entry():\np = lift.addr @g\nlift.free p\nlift.retval x0\n",
                    [4], 4, 4, (9, 4, 0, 0), lambda x: (0, 7), layout=sym_layout, data="g 0a0b\n"),
    ]
    # Recursive lproc: monotonic stack (R = 3 + 4 = 7 cells per call), depth and step accounting.
    rec = (O1 + "regs { r:reg[4] }\n^entry():\nr = lift.call @f (x0)\nlift.retval r\n"
           "lproc @f (n:reg[4]) (r:reg[4]) () () ()\nregs { z:reg[4], one:reg[4], m:reg[4] }\n^entry():\nz = lift.const.i4 0\n"
           "qp = lift.icmp.eq n z\nqbranch qp ^d ^m\n^m():\none = lift.const.i4 1\nm = lift.sub n one\nr = lift.call @f (m)\none = lift.add r one\nlift.ret one\n^d():\nlift.ret z\n").replace(
        "regs { z:reg[4], one:reg[4], m:reg[4] }", "regs { z:reg[4], one:reg[4], m:reg[4], qp:reg[1] }")
    rl = {"A_tab": 1, "H": 4, "S_cells": 8, "globals": {}, "heap_base": 1, "heap_limit_max": 0, "interface_regions": {}, "null_size": 1, "ptr_width": 8}
    # m' = n + r + 4 regs = 6, R_f = 9 cells; x+1 calls.
    out += [
        oracle_case("lproc-recursive-sum", rec, [4], 4, 4, (1000, 0, 200, 50), lambda x: (x, 0), layout=rl),
        oracle_case("lproc-depth-limit", rec, [4], 4, 4, (1000, 0, 200, 3), lambda x: (x, 0) if x + 1 <= 3 else (0, 3), layout=rl),
        oracle_case("lproc-stack-monotonic", rec, [4], 4, 4, (1000, 0, 27, 50), lambda x: (x, 0) if 9 * (x + 1) <= 27 else (0, 3), layout=rl),
        oracle_case("lproc-step-budget", rec, [4], 4, 4, (12, 0, 200, 50),
                    # root call 1; base level 3+ret=4; each recursive level 3+3+2=8; retval 1
                    lambda x: (x, 0) if 1 + 4 + 8 * x + 1 <= 12 else (0, 1), layout=rl),
    ]
    # Actions
    out += [
        action_case("action-cx", "action @a (t:wire mut) (c:wire in) () () ()\n^entry():\ncx c t\n", [1, 1], lambda t, c: (t ^ c, c)),
        action_case("action-controlled", "action @a (t:wire mut) (c:wire in) () () ()\n^entry():\ncx c t\n", [1, 1],
                    lambda w, t, c: (w, t ^ (w & c), c), inherited=1),
        action_case("action-qif", "action @a (t:wire mut, u:wire mut) (c:wire in) () () ()\n^entry():\nqif c {\nx t\n} {\nx u\n}\n", [1, 1, 1],
                    lambda t, u, c: (t ^ c, u ^ (1 - c), c)),
        action_case("action-qloop", "action @a (t:wire[2] mut) () () () ()\nregs {  } bounds {3}\n^entry():\nqloop 3 {\nx t[0]\ncx t[0] t[1]\n}\n", [2],
                    lambda t: (_loop3(t),)),
        action_case("action-qloop-while", "action @a (t:wire mut) (k:wire[2] in) () () ()\nregs {  } bounds {2}\n^entry():\nqloop 2 while k {\nx t\n}\n", [1, 2],
                    lambda t, k: (t ^ ((k & 1) ^ (1 if k == 3 else 0)), k)),
        action_case("action-qcopy-quncopy", "action @a (t:wire[2] mut) (s:wire[2] in) () () ()\n^entry():\nd = qcopy s\nqxor d t\nquncopy s d\n", [2, 2],
                    lambda t, s: (t ^ s, s)),
        action_case("action-qexch", "action @a (p:wire[2] mut, q:wire[2] mut) () () () ()\n^entry():\nqexch p q\n", [2, 2], lambda p, q: (q, p)),
        action_case("action-swap-mcx", "action @a (p:wire mut, q:wire mut) (c:wire[2] in) () () ()\n^entry():\nswap p q\nmcx (c) p\n", [1, 1, 2],
                    lambda p, q, c: (q ^ (1 if c == 3 else 0), p, c)),
        action_case("action-pred", "action @a (t:wire mut) (c:wire in, d:wire in) () () ()\n^entry():\nqpred c\nqpred d\nx t\nqunpred d\nqunpred c\n", [1, 1, 1],
                    lambda t, c, d: (t ^ (c & d), c, d)),
        action_case("action-hzh", "action @a (t:wire mut) () () () ()\n^entry():\nh t\ns t\ns t\nh t\n", [1], lambda t: (t ^ 1,)),
        action_case("action-ttdg", "action @a (t:wire mut) () () () ()\n^entry():\nt t\ntdg t\n", [1], lambda t: (t,)),
        action_case("action-borrow", "action @a (t:wire mut) (x:wire in) () () ()\n^entry():\nb = qborrow any 1\ncx x b[0]\ncx b[0] t\ncx x b[0]\ncx b[0] t\nqreturn b\n", [1, 1],
                    lambda t, x, b: (t ^ x, x, b), borrowed=[1]),
        action_case("action-call", "action @a (t:wire mut) (c:wire in) () () ()\n^entry():\naction_call @s (t) (c) () ()\naction_call @s (t) (c) () ()\naction_call @s (t) (c) () ()\naction @s (u:wire mut) (v:wire in) () () ()\n^entry():\ncx v u\n", [1, 1],
                    lambda t, c: (t ^ c, c)),
        action_case("action-adj-call", "action @a (t:wire mut) () () () ()\n^entry():\nadj action_call @s (t) () () ()\naction @s (u:wire mut) () () () ()\n^entry():\nh u\ns u\ns u\nh u\ns u\nsdg u\n", [1], lambda t: (t ^ 1,)),
        action_case("action-ctl-call", "action @a (t:wire mut) (c:wire in) () () ()\n^entry():\nctl c action_call @s (t) () () ()\naction @s (u:wire mut) () () () ()\n^entry():\nx u\n", [1, 1],
                    lambda t, c: (t ^ c, c)),
        # §4.5 sem through a pair, §5.3 reference over the full mut domain
        action_case("action-qinplace", "action @a (x:wire[2] mut) () () (tk:token) ()\n^entry():\nqinplace @g @h tk x\n"
                    "oracle @g (x:wire[2] in) (t:wire[2]) (f:wire[4]) () ()\nregs { o:reg[2], r:reg[2] }\n^entry():\no = lift.const.i2 1\nr = lift.add x o\nlift.retval r\n"
                    "oracle @h (x:wire[2] in) (t:wire[2]) (f:wire[4]) () ()\nregs { o:reg[2], r:reg[2] }\n^entry():\no = lift.const.i2 1\nr = lift.sub x o\nlift.retval r\n",
                    [2], lambda x: ((x + 1) & 3,), args=[["3", "0", "0", "0"]], budget=(3, 0, 0, 0), names=["x"]),
        action_case("action-oracle-call", "action @a (t:wire[2] mut, r:wire[4] mut) (x:wire[2] in) () (tk:token) ()\n^entry():\noracle_call @g tk (x) t r () ()\n"
                    "oracle @g (x:wire[2] in) (t:wire[2]) (f:wire[4]) () ()\nregs { o:reg[2], r:reg[2] }\n^entry():\no = lift.const.i2 1\nr = lift.add x o\nlift.retval r\n",
                    [2, 4, 2], lambda t, r, x: (t ^ ((x + 1) & 3), r, x), args=[["3", "0", "0", "0"]], budget=(3, 0, 0, 0), names=["t", "r", "x"]),
        action_case("action-oracle-call-short-token", "action @a (t:wire[2] mut, r:wire[4] mut) (x:wire[2] in) () (tk:token) ()\n^entry():\noracle_call @g tk (x) t r () ()\n"
                    "oracle @g (x:wire[2] in) (t:wire[2]) (f:wire[4]) () ()\nregs { o:reg[2], r:reg[2] }\n^entry():\no = lift.const.i2 1\nr = lift.add x o\nlift.retval r\n",
                    [2, 4, 2], lambda t, r, x: (t, r ^ 1, x), args=[["2", "0", "0", "0"]], budget=(3, 0, 0, 0), names=["t", "r", "x"]),
    ]
    return out


def _loop3(t):
    for _ in range(3):
        t ^= 1
        t ^= (t & 1) << 1
    return t
