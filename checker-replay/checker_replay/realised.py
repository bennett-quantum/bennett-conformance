"""Scheduled subjects: the left side of the K13 relation is the realised operation.

Replay computes reference semantics, so a scheduled table/program claim holds for
the realisation only if every generated gate is realised exactly (§4.4 ε = 0). This
module establishes that where it can be read off the source: action bodies built
from gates, qpred/qunpred, counted qloops and forward action calls, whose bound-library
expansion emits each source gate as one leaf. Everything else is unchecked with a
reason (§11.1 expansion, schemes, non-root contexts), never assumed.
"""
from fractions import Fraction
from .errors import Reject, Unchecked
from .objparse import parse_list

REF_LIBRARY = {"name": "ref", "version": "0.9",
               "hash": "5de4e4995350313e50217880ba1b95de39cd043283b726337d4894efb005d197"}
GATES = {'x': 0, 'cx': 1, 'ccx': 2, 'swap': 0, 'h': 0, 's': 0, 'sdg': 0, 't': 0, 'tdg': 0, 'cz': 1}


def _dec(v):
    if isinstance(v, bool) or not isinstance(v, (int, str, float)):
        raise Reject("E_SCHEDULE_INFEASIBLE", "11", "", f"non-decimal value {v!r}")
    return Fraction(str(v))


def _infeasible(msg):
    return Reject("E_SCHEDULE_INFEASIBLE", "11", "", msg)


def check_bindings(schedule, subject, closure_of_name, params_hash, budget):
    b = schedule["bindings"]
    for field, want in (("closure_hash", subject["closure_hash"]), ("variant", subject["variant"]),
                        ("inherited_controls", subject["inherited_controls"]), ("params_hash", params_hash)):
        if b.get(field) != want:
            raise _infeasible(f"schedule bindings.{field} is {b.get(field)!r}, the claim binds {want!r}")
    if closure_of_name.get(b.get("region")) != subject["closure_hash"]:
        raise _infeasible(f"schedule bindings.region {b.get('region')!r} does not name the subject")
    got = b.get("budget")
    if not isinstance(got, list) or [str(x) for x in got] != [str(x) for x in budget]:
        raise _infeasible(f"schedule budget {got!r} differs from the invocation budget {budget!r}")
    if b.get("expansion") != REF_LIBRARY:
        raise Unchecked(f"schedule binds expansion {b.get('expansion')!r}; only the published ref library is known", "11.1")
    if _dec(b.get("epsilon", 0)) != 0:
        raise Unchecked("schedule epsilon > 0: an inexact realisation has no basis relation; numerical/channel claims cover it", "12.1")


def _covers(entry, c):
    ctl = entry.get("controls", {})
    hi = ctl.get("max")
    return ctl.get("min", 0) <= c and (hi == "unbounded" or (isinstance(hi, int) and c <= hi))


def _native_exact(target, op, c):
    """First target gates entry covering (op, c), per §8.3 matching with eps = 0."""
    classes = target.get("classes", {})
    if len(classes) != 1:
        raise Unchecked("multi-class target: operand class resolution is not replayed", "8.3")
    only = next(iter(classes))
    for g in target.get("gates", []):
        if g.get("op") != op or not _covers(g, c):
            continue
        if "*" not in g.get("classes", []) and only not in g.get("classes", []):
            continue
        eps = g.get("epsilon")
        if isinstance(eps, bool) or not isinstance(eps, (int, float, str)):
            raise Unchecked(f"gate {op} epsilon is the expression {eps!r}; §8.1 evaluation is not replayed", "8.1")
        try:
            value = Fraction(str(eps))
        except ValueError:
            raise Unchecked(f"gate {op} epsilon is the expression {eps!r}; §8.1 evaluation is not replayed", "8.1") from None
        if value == 0:
            return True
    return False


class _Walk:
    def __init__(self, obj, closure, schedule, target):
        self.obj, self.closure, self.schedule, self.target = obj, closure, schedule, target
        self.gate_entries = [e for e in schedule["entries"] if e.get("kind") == "gate"]
        self.points = []
        self.leaves = 0

    def visit(self, name, chain, pred, adj=False):
        fn = self.obj.functions[name]
        sym = self.closure[name]
        for bname, _params, instrs in fn.blocks:
            ordinal = [0]
            self.seq(name, sym, bname, instrs, ordinal, chain, pred, [], [])

    def seq(self, name, sym, bname, instrs, ordinal, chain, pred, iters, stack):
        for ins in instrs:
            point = f"{bname}:{ordinal[0]}"
            ordinal[0] += 1
            op = ins.op
            here = {"symbol": sym, "point": point, "iteration": list(iters), "instance": []}
            if op == 'qloop' and len(ins.operands) == 1:
                saved = ordinal[0]
                for i in range(int(ins.operands[0])):
                    ordinal[0] = saved
                    self.seq(name, sym, bname, ins.nested[0], ordinal, chain, pred, iters + [i], stack)
                if not int(ins.operands[0]):
                    self._count(ins.nested[0], ordinal)
            elif op == 'qpred':
                stack.append(ins.operands[0])
            elif op == 'qunpred':
                stack.pop()
            elif op in GATES or op == 'mcx':
                own = GATES[op] if op != 'mcx' else len(parse_list(ins.operands[0]))
                self.leaf(op, own + pred + len(stack), chain + [here])
            elif op == 'action_call' or (op == 'ctl' and ins.operands[1] == 'action_call'):
                ops = ins.operands if op == 'action_call' else ins.operands[2:]
                extra = 0 if op == 'action_call' else 1
                cname = ops[0][1:]
                if self.obj.functions.get(cname) is None:
                    raise Unchecked(f"call to @{cname}: linked realisation is not replayed", "9.3")
                edge = dict(here, call_index=0)
                self.points.append(chain + [edge])
                self.visit(cname, chain + [edge], pred + len(stack) + extra)
            else:
                raise Unchecked(f"realising {op!r} needs the §11.1 expansion of the bound library, not replayed", "11.1")

    def _count(self, instrs, ordinal):
        for ins in instrs:
            ordinal[0] += 1
            for sub in ins.nested:
                self._count(sub, ordinal)

    @staticmethod
    def _element_matches(sel, el):
        if sel.get("symbol") != el["symbol"] or sel.get("point") != el["point"]:
            return False
        it = sel.get("iteration", "*")
        if it != "*" and it != el["iteration"]:
            return False
        inst = sel.get("instance", "*")
        if inst != "*" and inst != el["instance"]:
            return False
        ci = sel.get("call_index", "*")
        if ci != "*" and ci != el.get("call_index"):
            return False
        return True

    def matches(self, entry, chain):
        path = entry.get("path", [])
        if len(path) > len(chain) or not all(self._element_matches(s, e) for s, e in zip(path, chain)):
            return False
        if entry.get("step", "*") != "*" or entry.get("phase", "*") != "*":
            return False  # §11: a level with no oracle step matches only wildcards
        return entry.get("generated", "*") in ("*", 0)

    @staticmethod
    def specificity(e):
        return (len(e.get("path", [])), sum(1 for s in e.get("path", []) if s.get("iteration", "*") != "*"),
                e.get("step", "*") != "*", e.get("phase", "*") != "*", e.get("generated", "*") != "*")

    def leaf(self, op, c, chain):
        self.leaves += 1
        self.points.append(chain)
        hits = [e for e in self.gate_entries if self.matches(e, chain)]
        chosen = None
        if hits:
            top = max(self.specificity(e) for e in hits)
            best = [e for e in hits if self.specificity(e) == top]
            if len(best) > 1:
                raise _infeasible("§11: multiple maximal matching schedule entries")
            chosen = best[0]
        if chosen is not None:
            g = chosen.get("gate")
            if not isinstance(g, dict) or "realisation" not in g:
                raise _infeasible(f"gate entry without a realisation: {chosen!r}")
            if _dec(g.get("epsilon_share", 0)) != 0:
                raise Unchecked("nonzero epsilon_share: an inexact gate has no basis relation", "4.4")
            if g["realisation"] != "native":
                raise Unchecked(f"gate realised by scheme {g['realisation']!r}: scheme correctness is not replayed", "8.3")
            if not _native_exact(self.target, op, c):
                raise _infeasible(f"native {op} with {c} total controls has no exact target entry")
            return
        if _native_exact(self.target, op, c):
            return
        if self.target.get("schemes"):
            raise Unchecked(f"default for {op} with {c} controls is a scheme: not replayed", "8.3")
        raise _infeasible(f"gate {op} with {c} total controls matches no target entry")


def establish_exact(obj, closure, schedule, target, subject_name, inherited):
    """Return the number of source gate leaves shown exact; raise otherwise."""
    walk = _Walk(obj, closure, schedule, target)
    walk.visit(subject_name, [], inherited)
    for e in schedule["entries"]:
        if e.get("path") and not any(
                len(e["path"]) <= len(p) and all(walk._element_matches(s, x) for s, x in zip(e["path"], p))
                for p in walk.points):
            raise _infeasible(f"entry path selects no generated operation: {e.get('path')!r}")
    return walk.leaves
