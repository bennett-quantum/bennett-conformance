"""Invocations of an object derived from its signatures (§9.1), deterministic and independent of executors.

Every symbol is either driven or yields a counted skip with a reason; `check` and `hash` always run.
"""
import re

import engine

ENTROPY = "6162636465666768"
DRAWS = "0.25,0.75,0.5,0.125,0.875,0.0625,0.9375,0.375"
ORACLE_BUDGETS = ("0", "1", "8", "256", "100000")
MAX_BASIS_WIRES = 256


def _int_values(width):
    return ["0", "1", str((1 << width) - 1)]


def _type_value(t, which):
    """Spelling of a classical argument of type t for vector index `which` in 0..2."""
    if t == "f64":
        return ["f64:0.5", "f64:-0.0", "f64:1.5"][which]
    if t == "token":
        return ["token:64", "token:0", "token:256"][which]
    if t == "ptr":
        return ["0", "1", "16"][which]
    m = re.fullmatch(r"i(\d+)", t)
    if m:
        return _int_values(int(m.group(1)))[which]
    m = re.fullmatch(r"qreg\[(\d+)\]", t)
    if m:
        n = int(m.group(1))
        return "qreg:" + {0: "0" * n, 1: "1" * n, 2: ("10" * n)[:n]}[which]
    raise ValueError("no spelling for classical type %r" % t)


def _arg_opts(types, which):
    out = []
    for t in types:
        out += ["--arg", _type_value(t, which)]
    return out


def _sig_mem_bits(mem):
    return sum((1 << m["A"]) * m["W"] for m in mem)


def _basis_vectors(n):
    if n == 0:
        return [""]
    return ["0" * n, "1" * n, ("10" * n)[:n]]


def for_object(corpus, base_id, runner, object_path=None, object_text=None, o=None, cwd=None):
    """Yield the driven Cases of a parsed object; report what is not driven through runner.skip."""
    def case(suffix, argv, tape=False):
        return engine.Case(corpus, "%s/%s" % (base_id, suffix), ["%s" % a for a in argv],
                           object_path=object_path, object_text=object_text, cwd=cwd, tape=tape)

    yield case("check", ["check", "$OBJECT"])
    yield case("hash", ["hash", "$OBJECT"])
    for name, sym in sorted(o.symbol_table().items()):
        kind, sig = sym["kind"], sym["signature"]
        if kind == "data":
            runner.skip(corpus, "%s/%s" % (base_id, name), "data symbol: no execution command (identity only)")
        elif kind == "lproc":
            runner.skip(corpus, "%s/%s" % (base_id, name), "lproc: reached only through its callers (§3.7)")
        elif kind in ("cc", "r"):
            yield from _classical(case, name, kind, sig)
        elif kind == "oracle":
            yield from _oracle(case, name, sig)
        elif kind in ("action", "perm_action"):
            yield from _action(case, name, sig, runner, corpus, base_id)
        else:
            raise ValueError("unknown symbol kind %r in %s" % (kind, base_id))


def _classical(case, name, kind, sig):
    common = ["--entropy", ENTROPY, "--draws", DRAWS]
    for which in range(3 if sig["params"] else 1):
        args = _arg_opts(sig["params"], which)
        yield case("%s/run-cc/%d" % (name, which), ["run-cc", "$OBJECT", name] + args + common, tape=True)
        yield case("%s/reverse/%d" % (name, which), ["reverse", "$OBJECT", name] + args + common, tape=True)
        yield case("%s/reverse-prefix/%d" % (name, which),
                   ["reverse", "$OBJECT", name, "--steps", "1"] + args + common, tape=True)
        if kind == "cc":
            yield case("%s/run-program/%d" % (name, which), ["run-program", "$OBJECT", name] + args + common, tape=True)
            yield case("%s/run-program-reverse/%d" % (name, which),
                       ["run-program", "$OBJECT", name, "--reverse"] + args + common, tape=True)


def _oracle(case, name, sig):
    widths, targs = sig["in"], sig["args"]
    for which in range(3 if widths else 1):
        inputs = ",".join(_int_values(w)[which] for w in widths)
        args = _arg_opts(targs, which)
        for budget in ORACLE_BUDGETS:
            yield case("%s/run-oracle/%d/B%s" % (name, which, budget),
                       ["run-oracle", "$OBJECT", name, "--input", inputs, "--budget", budget] + args)
        for budget in ("8", "256"):
            yield case("%s/reverse/%d/B%s" % (name, which, budget),
                       ["reverse", "$OBJECT", name, "--input", inputs, "--budget", budget] + args)
    yield case("%s/step-check" % name, ["step-check", "$OBJECT", name, "--samples", "4", "--seed", "1,2,3,4",
                                        "--budget", "64"] + _arg_opts(targs, 0))


def _action(case, name, sig, runner=None, corpus=None, base_id=None):
    n = sum(sig["mut"]) + sum(sig["in"]) + _sig_mem_bits(sig["mem"])
    if n > MAX_BASIS_WIRES:
        runner.skip(corpus, "%s/%s" % (base_id, name), "action interface wider than %d wires: no basis vectors" % MAX_BASIS_WIRES)
        return
    args = _arg_opts(sig["args"], 0)
    for i, basis in enumerate(_basis_vectors(n)):
        for adjoint in (False, True):
            extra = ["--adjoint"] if adjoint else []
            tag = "%d%s" % (i, "adj" if adjoint else "")
            yield case("%s/run-action/%s" % (name, tag), ["run-action", "$OBJECT", name, "--basis", basis] + extra + args)
            yield case("%s/reverse/%s" % (name, tag), ["reverse", "$OBJECT", name, "--basis", basis] + extra + args)
    if n >= 2:
        zero, one = "0" * n, "1" + "0" * (n - 1)
        state = "%s=0.5;%s=0,0.5" % (zero, one)
        yield case("%s/run-action/state" % name, ["run-action", "$OBJECT", name, "--state", state] + args)
        yield case("%s/reverse/state" % name, ["reverse", "$OBJECT", name, "--state", state] + args)
