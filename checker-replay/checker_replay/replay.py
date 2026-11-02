"""Exhaustive-claim replay of a §12 certificate against its object.

Every exhaustive claim is recomputed over its full declared domain from the object
alone. Anything not replayed is reported `unchecked` with a reason, never accepted.
"""
import itertools, re
from fractions import Fraction
from . import hashes as H, jcs, realised
from .action import compile_action
from .errors import ObjectError, Reject, Unchecked
from .objparse import parse_list, region_shape, type_width
from .oracle import oracle_map, oracle_payload

HEX64 = re.compile(r'^[0-9a-f]{64}$')
DEC = re.compile(r'^(0|[1-9][0-9]*)$')
F64 = re.compile(r'^0x[0-9a-f]{16}$')
DEFAULT_MAX_ELEMENTS = 1 << 20
MAX_MATERIALISED_A = 20
SCOPE_NOTES = [
    "§12.1 sampled, structural, numerical and channel claims: reported unchecked per claim",
    "§12.3 resources, gates_used, estimate, frame, cleanup, error, composition: not recomputed",
    "§12.3 verification.borrow_frames, unitary, channel, oracle, fault_rate indices: not recomputed",
    "§4.5 qinplace coverage by inverse_pairs entries: not recomputed (dirty t/f is detected in replay)",
    "§16/suite F object well-formedness: only rules the replay relies on are enforced",
    "§9.3 linked libraries: symbols outside the given object are unchecked",
]


def _r(code, section, path, msg, witness=None):
    return Reject(code, section, path, msg, witness)


def _nat(v, path, section="12.2"):
    """A nonnegative integer as a JSON number or a canonical decimal string (§9.1)."""
    if isinstance(v, bool):
        raise _r("E_REPLAY_SCHEMA", section, path, f"{v!r} is not an integer")
    if isinstance(v, int) and v >= 0:
        return v
    if isinstance(v, str) and DEC.match(v):
        return int(v)
    raise _r("E_REPLAY_SCHEMA", section, path, f"{v!r} is not a nonnegative integer")


def _closed(d, required, optional, path, section):
    if not isinstance(d, dict):
        raise _r("E_REPLAY_SCHEMA", section, path, f"expected an object, got {type(d).__name__}")
    missing = set(required) - set(d)
    if missing:
        raise _r("E_REPLAY_SCHEMA", section, path, f"missing fields {sorted(missing)}")
    unknown = set(d) - set(required) - set(optional)
    if unknown:
        raise _r("E_REPLAY_SCHEMA", section, path, f"unknown fields {sorted(unknown)}")


def _list(v, path, section):
    if not isinstance(v, list):
        raise _r("E_REPLAY_SCHEMA", section, path, f"expected an array, got {type(v).__name__}")
    return v


def _canon_dec(v, path, section="12.3"):
    if not isinstance(v, str) or not DEC.match(v):
        raise _r("E_REPLAY_INVOCATION", section, path, f"{v!r} is not a canonical nonnegative decimal string")
    return int(v)


class _Subject:
    """Interface layout of the claim's subject, independent of its kind."""

    def __init__(self, obj, name, inherited):
        self.obj, self.name, self.inherited = obj, name, inherited
        self.fn = obj.functions.get(name)
        if self.fn is None:
            raise Unchecked(f"subject @{name} has no body in this object", "9.3")
        self.kind = self.fn.kind
        g = self.fn.groups
        pw = obj.ptr_width
        if self.kind == 'oracle':
            self.bundles = list(g[0]) + list(g[1]) + list(g[2])
            self.mem, self.params = g[3], g[4]
            self.roles = ['in'] * len(g[0]) + ['tgt'] + ['fault']
        elif self.kind in ('action', 'perm_action'):
            self.bundles = list(g[0]) + list(g[1])
            self.mem, self.params = g[2], g[3]
            self.roles = ['mut'] * len(g[0]) + ['in'] * len(g[1])
        else:
            raise _r("E_REPLAY_SUBJECT", "12.1", "subject.closure_hash",
                     f"@{name} is a {self.kind}, not a region (§5)")
        self.widths = [type_width(t, pw) for _n, t, _s in self.bundles]
        self.shapes = [region_shape(t) for _n, t, _s in self.mem]
        self.ptypes = [t for _n, t, _s in self.params]
        self.prog = None
        self.borrow_widths = []

    def bind(self, args):
        self.args = args
        if self.kind != 'oracle':
            self.prog = compile_action(self.obj, self.name, args, self.inherited)
            self.borrow_widths = self.prog.borrow_widths
        else:
            self.borrow_widths = self._oracle_borrow_requirement()

    def _oracle_borrow_requirement(self):
        """Oracle bodies have no qborrow row; only a perm_call callee could introduce a borrow."""
        for inst in _closure_rows(self.obj, self.name):
            if inst.op == 'lift.perm_call':
                cname = inst.operands[0][1:]
                if self.obj.functions.get(cname) is None:
                    raise Unchecked(f"lift.perm_call @{cname} is not defined here", "9.3")
                if any(r.op == 'qborrow' for _b, _p, rs in self.obj.functions[cname].blocks for r in _flat(rs)):
                    raise Unchecked(f"@{cname} borrows wires inside an oracle's lifted call; its tuple position "
                                    "is a contract question (K13 borrowed-oracle witness)", "12.1")
        return []

    def run(self, budget, bundles, borrowed, regions):
        """Reference semantics at the bound invocation: the K13 after tuple."""
        if self.kind == 'oracle':
            ctl = bundles[:self.inherited]
            rest = bundles[self.inherited:]
            nin = len(self.fn.groups[0])
            ins, tgt, fault = rest[:nin], rest[nin], rest[nin + 1]
            if all(ctl):
                tgt, fault = oracle_map(self.obj, self.name, ins, tgt, fault, regions, self.args, budget)
            return (ctl + ins + [tgt, fault], list(borrowed), [list(r) for r in regions])
        return self.prog.run(bundles, borrowed, regions)


def _flat(instrs):
    for i in instrs:
        yield i
        for sub in i.nested:
            yield from _flat(sub)


def _closure_rows(obj, name):
    seen, work = set(), [name]
    while work:
        n = work.pop()
        if n in seen or obj.functions.get(n) is None:
            continue
        seen.add(n)
        for _b, _p, rows in obj.functions[n].blocks:
            for r in _flat(rows):
                yield r
                if r.op == 'lift.call':
                    work.append(r.operands[0][1:])


def _validate_budget(obj, budget, path):
    b, heap, stack, depth = budget
    lay = obj.layout
    if lay is None:
        if heap or stack or depth:
            raise _r("E_REPLAY_INVOCATION", "5.4", path, f"memoryless object requires heap=stack=depth=0, got {budget}")
        return
    if heap > lay["heap_limit_max"]:
        raise _r("E_REPLAY_INVOCATION", "5.4", path, f"heap_limit {heap} > heap_limit_max {lay['heap_limit_max']}")
    if stack > 1 << lay["S_cells"]:
        raise _r("E_REPLAY_INVOCATION", "5.4", path, f"stack_limit {stack} > 2^S_cells")


def _resolve_value(obj, typ, v, path):
    """K13 typed encoding of one argument, resolved to its semantic value."""
    if typ == 'token':
        if not isinstance(v, list) or len(v) != 4:
            raise _r("E_REPLAY_INVOCATION", "12.3", path, f"token value {v!r} is not a four-component tuple")
        t = tuple(_canon_dec(x, path) for x in v)
        _validate_budget(obj, t, path)
        return t
    if typ == 'f64':
        if not isinstance(v, str) or not F64.match(v):
            raise _r("E_REPLAY_INVOCATION", "12.3", path, f"f64 value {v!r} is not 0x + 16 lowercase hex digits")
        return int(v, 16)
    w = type_width(typ, obj.ptr_width)
    n = _canon_dec(v, path)
    if n >= 1 << w:
        raise _r("E_REPLAY_INVOCATION", "12.3", path, f"{v} does not fit {typ}")
    return n


def _resolve_invocation(obj, inv, ptypes, path):
    _closed(inv, ["args", "budget"], [], path, "12.3")
    args = _list(inv["args"], path + ".args", "12.3")
    bud = _list(inv["budget"], path + ".budget", "12.3")
    if len(bud) != 4:
        raise _r("E_REPLAY_INVOCATION", "12.3", path + ".budget", "all four budget components are required")
    budget = tuple(_canon_dec(x, path + ".budget") for x in bud)
    _validate_budget(obj, budget, path + ".budget")
    if len(args) != len(ptypes):
        raise _r("E_REPLAY_INVOCATION", "12.3", path + ".args", f"{len(args)} values for {len(ptypes)} parameters")
    vals = [_resolve_value(obj, t, v, f"{path}.args[{i}]") for i, (t, v) in enumerate(zip(ptypes, args))]
    return vals, budget


class _Domain:
    """§12.2 resolution. `comps` are (identity, lo, hi) in enumeration order."""

    def __init__(self):
        self.comps = []
        self.seen = set()

    def add(self, ident, lo, hi, width, path):
        if ident in self.seen:
            raise _r("E_REPLAY_DOMAIN", "12.2", path, f"component {ident} occurs twice")
        self.seen.add(ident)
        if lo > hi or hi >= 1 << width:
            raise _r("E_REPLAY_DOMAIN", "12.2", path, f"range [{lo},{hi}] invalid for width {width}")
        self.comps.append((ident, lo, hi))


def _range_entry(e, path, need_width):
    keys = ["name", "width", "lo", "hi"]
    _closed(e, keys, [], path, "12.2")
    if not isinstance(e["name"], str):
        raise _r("E_REPLAY_SCHEMA", "12.2", path + ".name", "name must be a string")
    w = _nat(e["width"], path + ".width")
    if need_width is not None and w != need_width:
        raise _r("E_REPLAY_DOMAIN", "12.2", path + ".width", f"width {w}, the component has width {need_width}")
    return w, _nat(e["lo"], path + ".lo"), _nat(e["hi"], path + ".hi")


def _resolve_domain(subj, dom, *, inverse_pair=False, path="domain"):
    _closed(dom, ["bundles", "borrowed", "regions"], ["args"] if inverse_pair else [], path, "12.2")
    out = _Domain()
    names = {n: i for i, (n, _t, _s) in enumerate(subj.bundles)}
    entries = _list(dom["bundles"], path + ".bundles", "12.2")
    c = subj.inherited
    if len(entries) < c:
        raise _r("E_REPLAY_DOMAIN", "12.2", path + ".bundles", f"all {c} inherited controls are required")
    for i, e in enumerate(entries):
        p = f"{path}.bundles[{i}]"
        if i < c:
            w, lo, hi = _range_entry(e, p, 1)
            out.add(("ctl", i), lo, hi, w, p)
            continue
        if not isinstance(e, dict) or not isinstance(e.get("name"), str) or e["name"] not in names:
            raise _r("E_REPLAY_DOMAIN", "12.2", p, f"{e!r} does not name an interface bundle of the subject")
        idx = names[e["name"]]
        if inverse_pair and subj.roles[idx] != 'in':
            raise _r("E_REPLAY_DOMAIN", "12.1", p, "inverse-pair bundles range over g's `in` slots only")
        w, lo, hi = _range_entry(e, p, subj.widths[idx])
        out.add(("bundle", idx), lo, hi, w, p)
    for k, e in enumerate(_list(dom["borrowed"], path + ".borrowed", "12.2")):
        p = f"{path}.borrowed[{k}]"
        if k >= len(subj.borrow_widths):
            raise _r("E_REPLAY_DOMAIN", "12.2", p, f"position {k} is beyond the checked borrow requirement "
                     f"({len(subj.borrow_widths)})")
        w, lo, hi = _range_entry(e, p, subj.borrow_widths[k])
        out.add(("borrowed", k), lo, hi, w, p)
    rnames = {n: i for i, (n, _t, _s) in enumerate(subj.mem)}
    for ri, e in enumerate(_list(dom["regions"], path + ".regions", "12.2")):
        p = f"{path}.regions[{ri}]"
        _closed(e, ["name", "cells"], [], p, "12.2")
        if not isinstance(e["name"], str) or e["name"] not in rnames:
            raise _r("E_REPLAY_DOMAIN", "12.2", p, f"{e['name']!r} is not an interface region")
        r = rnames[e["name"]]
        A, W = subj.shapes[r]
        if e["cells"] == "all":
            if A > MAX_MATERIALISED_A:
                raise Unchecked(f"host limit: region {e['name']} has 2^{A} cells", "12.2")
            for i in range(1 << A):
                out.add(("cell", r, i), 0, (1 << W) - 1, W, p)
        else:
            for j, cell in enumerate(_list(e["cells"], p + ".cells", "12.2")):
                cp = f"{p}.cells[{j}]"
                _closed(cell, ["index", "lo", "hi"], [], cp, "12.2")
                idx = _nat(cell["index"], cp + ".index")
                if idx >= 1 << A:
                    raise _r("E_REPLAY_DOMAIN", "12.2", cp + ".index", f"index {idx} outside 2^{A}")
                out.add(("cell", r, idx), _nat(cell["lo"], cp + ".lo"), _nat(cell["hi"], cp + ".hi"), W, cp)
    return out


def _full_bits(subj):
    """log2 |D|: every prepended control, bundle, borrowed position and region cell."""
    bits = subj.inherited + sum(subj.widths) + sum(subj.borrow_widths)
    for A, W in subj.shapes:
        if A > MAX_MATERIALISED_A:
            raise Unchecked(f"host limit: region of 2^{A} cells", "12.2")
        bits += W << A
    return bits


def _count(dom):
    n = 1
    for _i, lo, hi in dom.comps:
        n *= hi - lo + 1
    return n


def _covers_d(subj, dom):
    want = {("ctl", i) for i in range(subj.inherited)}
    want |= {("bundle", i) for i in range(len(subj.widths))}
    want |= {("borrowed", k) for k in range(len(subj.borrow_widths))}
    for r, (A, _W) in enumerate(subj.shapes):
        want |= {("cell", r, i) for i in range(1 << A)}
    if {c[0] for c in dom.comps} != want:
        return False
    for ident, lo, hi in dom.comps:
        if ident[0] == "ctl":
            w = 1
        elif ident[0] == "bundle":
            w = subj.widths[ident[1]]
        elif ident[0] == "borrowed":
            w = subj.borrow_widths[ident[1]]
        else:
            w = subj.shapes[ident[1]][1]
        if lo != 0 or hi != (1 << w) - 1:
            return False
    return True


def _elements(subj, dom):
    """Yield (bundles, borrowed, regions) in enumeration order, omitted components at zero."""
    ranges = [range(lo, hi + 1) for _i, lo, hi in dom.comps]
    idents = [c[0] for c in dom.comps]
    nb, nbor = subj.inherited + len(subj.widths), len(subj.borrow_widths)
    for values in itertools.product(*ranges):
        bundles, borrowed = [0] * nb, [0] * nbor
        regions = [[0] * (1 << A) for A, _W in subj.shapes]
        for ident, v in zip(idents, values):
            if ident[0] == "ctl":
                bundles[ident[1]] = v
            elif ident[0] == "bundle":
                bundles[subj.inherited + ident[1]] = v
            elif ident[0] == "borrowed":
                borrowed[ident[1]] = v
            else:
                regions[ident[1]][ident[2]] = v
        yield bundles, borrowed, regions


def _tuple_str(t):
    return {"bundles": [str(x) for x in t[0]], "borrowed": [str(x) for x in t[1]],
            "regions": [[str(x) for x in r] for r in t[2]]}


def _parse_tuple(t, subj, path):
    _closed(t, ["bundles", "borrowed", "regions"], [], path, "12.1")

    def num(v, w, p):
        if not isinstance(v, str) or not DEC.match(v):
            raise _r("E_REPLAY_TABLE", "12.1", p, f"{v!r} is not a canonical decimal string")
        n = int(v)
        if n >= 1 << w:
            raise _r("E_REPLAY_TABLE", "12.1", p, f"{v} is out of range for width {w}")
        return n
    bw = [1] * subj.inherited + subj.widths
    bund = _list(t["bundles"], path + ".bundles", "12.1")
    if len(bund) != len(bw):
        raise _r("E_REPLAY_TABLE", "12.1", path + ".bundles", f"{len(bund)} values, the tuple has {len(bw)} bundles")
    bor = _list(t["borrowed"], path + ".borrowed", "12.1")
    if len(bor) != len(subj.borrow_widths):
        raise _r("E_REPLAY_TABLE", "12.1", path + ".borrowed", f"{len(bor)} values for {len(subj.borrow_widths)} borrowed")
    regs = _list(t["regions"], path + ".regions", "12.1")
    if len(regs) != len(subj.shapes):
        raise _r("E_REPLAY_TABLE", "12.1", path + ".regions", f"{len(regs)} regions for {len(subj.shapes)}")
    out_r = []
    for i, (r, (A, W)) in enumerate(zip(regs, subj.shapes)):
        _list(r, f"{path}.regions[{i}]", "12.1")
        if len(r) != 1 << A:
            raise _r("E_REPLAY_TABLE", "12.1", f"{path}.regions[{i}]", f"{len(r)} cells for 2^{A}")
        out_r.append([num(v, W, f"{path}.regions[{i}][{j}]") for j, v in enumerate(r)])
    return ([num(v, w, f"{path}.bundles[{i}]") for i, (v, w) in enumerate(zip(bund, bw))],
            [num(v, w, f"{path}.borrowed[{i}]") for i, (v, w) in enumerate(zip(bor, subj.borrow_widths))],
            out_r)


class Checker:
    def __init__(self, cert, obj, object_text, target=None, schedule=None, max_elements=DEFAULT_MAX_ELEMENTS):
        self.cert, self.obj, self.text = cert, obj, object_text
        self.target = target
        self.schedule = schedule if schedule is not None else obj.schedule
        self.max_elements = max_elements
        self.hashes = H.compute_hashes(obj)
        self.by_closure = {v[1]: k for k, v in self.hashes.items()}
        self.closure_of_name = {k: v[1] for k, v in self.hashes.items()}

    # ---- document level
    def run(self):
        errors, regions = [], {}
        cert = self.cert
        if not isinstance(cert, dict):
            return _final("reject", [_diag("E_REPLAY_SCHEMA", "12.3", "$", "certificate is not an object")], {}, self)
        versions = cert.get("versions")
        if self.obj.isa != "0.9" or not isinstance(versions, dict) or versions.get("cert") != "0.9":
            return _final("unchecked", [], {}, self,
                          reason=f"object isa={self.obj.isa}, cert versions {versions!r}: only cert 0.9 is replayed")
        bindings = cert.get("bindings")
        try:
            self.check_bindings(bindings)
        except Reject as e:
            errors.append(e.diag)
        recs = cert.get("regions")
        if not isinstance(recs, dict):
            errors.append(_diag("E_REPLAY_SCHEMA", "12.3", "regions", "regions must be an object"))
            recs = {}
        for key, rec in recs.items():
            regions[key] = self.check_region(key, rec)
        return _final(None, errors, regions, self)

    def check_bindings(self, b):
        _closed(b, ["object_hash", "interface_hash", "closure_hash", "reference_program_hash",
                    "dialect_mapping_version", "target_hash"], ["schedule_hash", "invocation"], "bindings", "12.3")
        got = H.object_hash(self.text)
        if b["object_hash"] != got:
            raise _r("E_REPLAY_BINDING", "9.2; 12.3", "bindings.object_hash", f"recomputed {got}")
        name = self.by_closure.get(b["closure_hash"])
        if name is None:
            raise _r("E_REPLAY_BINDING", "12.3", "bindings.closure_hash", f"{b['closure_hash']} names no symbol")
        iface = self.hashes[name][0]
        for field in ("interface_hash", "reference_program_hash"):
            if b[field] != iface:
                raise _r("E_REPLAY_BINDING", "9.2", f"bindings.{field}", f"recomputed {iface}")
        for s in self.obj.symbols:
            if self.hashes[s["name"]][0] != s["interface_hash"]:
                raise _r("E_REPLAY_OBJECT", "9.2", f"symbols.{s['name']}", "published interface_hash is wrong")
        if self.target is not None:
            t = H.sha256hex(jcs.canonical(self.target))
            if b["target_hash"] != t:
                raise _r("E_REPLAY_BINDING", "12.3", "bindings.target_hash", f"recomputed {t}")
        if "schedule_hash" in b:
            if self.schedule is None:
                raise _r("E_REPLAY_BINDING", "12.3", "bindings.schedule_hash",
                         "bound, but no schedule is embedded or supplied")
            s = H.sha256hex(jcs.canonical(self.schedule))
            if b["schedule_hash"] != s:
                raise _r("E_REPLAY_BINDING", "9.2; 12.3", "bindings.schedule_hash", f"recomputed {s}")
            if "invocation" not in b:
                raise _r("E_REPLAY_INVOCATION", "12.3", "bindings.invocation", "a scheduled certificate must carry it")

    def check_region(self, key, rec):
        out = {"errors": [], "claims": [], "unchecked": []}
        if self.by_closure.get(key) is None:
            out["errors"].append(_diag("E_REPLAY_BINDING", "12.3", f"regions.{key}", "key names no symbol's closure hash"))
        if not isinstance(rec, dict) or not isinstance(rec.get("claims"), list):
            out["errors"].append(_diag("E_REPLAY_SCHEMA", "12.3", f"regions.{key}.claims", "claims must be an array"))
            return out
        for i, claim in enumerate(rec["claims"]):
            out["claims"].append(self.check_claim(key, rec, i, claim))
        return out

    # ---- claim level
    def check_claim(self, key, rec, index, claim):
        kind = claim.get("kind") if isinstance(claim, dict) else None
        res = {"index": index, "kind": kind if isinstance(kind, str) else None}
        try:
            if kind == "exhaustive":
                res.update(self.exhaustive(rec, index, claim))
            elif kind in ("sampled", "structural", "numerical", "channel"):
                raise Unchecked(f"{kind} claims are outside this replay (exhaustive only)", "12.1")
            else:
                raise _r("E_REPLAY_SCHEMA", "12.1", "kind", f"unknown claim kind {kind!r}")
        except Reject as e:
            res.update(outcome="reject", errors=[e.diag])
        except ObjectError as e:
            res.update(outcome="reject", errors=[_diag("E_REPLAY_OBJECT", e.section, "object", e.message)])
        except Unchecked as e:
            res.update(outcome="unchecked", reason=e.message, section=e.section)
        return res

    def subject_of(self, claim):
        s = claim["subject"]
        _closed(s, ["closure_hash", "variant", "inherited_controls"], ["schedule_hash"], "subject", "12.1")
        if not isinstance(s["closure_hash"], str) or not HEX64.match(s["closure_hash"]):
            raise _r("E_REPLAY_SUBJECT", "12.1", "subject.closure_hash", f"{s['closure_hash']!r} is not 64-hex")
        if not isinstance(s["variant"], str) or not s["variant"]:
            raise _r("E_REPLAY_SUBJECT", "12.3", "subject.variant", "a variant is a nonempty string")
        c = s["inherited_controls"]
        if isinstance(c, bool) or not isinstance(c, int) or c < 0:
            raise _r("E_REPLAY_SUBJECT", "12.1", "subject.inherited_controls", f"{c!r}")
        name = self.by_closure.get(s["closure_hash"])
        if name is None:
            raise _r("E_REPLAY_SUBJECT", "12.1", "subject.closure_hash", "names no closure of this object")
        if "schedule_hash" in s:
            sh = s["schedule_hash"]
            if not isinstance(sh, str) or sh != (self.cert.get("bindings") or {}).get("schedule_hash"):
                raise _r("E_REPLAY_BINDING", "12.1", "subject.schedule_hash", f"{sh!r} differs from bindings.schedule_hash")
        return name, c, s

    def exhaustive(self, rec, index, claim):
        _closed(claim, ["kind", "domain", "expected", "subject"], ["label", "invocation"], "", "12.1")
        label = claim.get("label")
        if "label" in claim and label not in ("proof", "evidence"):
            raise _r("E_REPLAY_LABEL", "12.2", "label", f"{label!r} is neither proof nor evidence")
        expected = claim["expected"]
        name, c, s = self.subject_of(claim)
        if expected == "inverse_pair":
            return self.inverse_pair(rec, index, claim, name, s)
        return self.relation(claim, name, c, s, expected, label)

    def relation(self, claim, name, c, s, expected, label):
        subj = _Subject(self.obj, name, c)
        scheduled = "schedule_hash" in s
        if expected == "oracle":
            if subj.kind != 'oracle':
                raise _r("E_REPLAY_EXPECTED", "12.1", "expected", '"oracle" compares to §5.2\'s oracle map; the subject is not an oracle')
            if not scheduled:
                raise _r("E_REPLAY_EXPECTED", "12.1", "subject.schedule_hash", 'expected "oracle" requires schedule_hash')
        elif not isinstance(expected, dict) or len(expected) != 1 or not ({"table", "program"} & set(expected)):
            raise _r("E_REPLAY_EXPECTED", "12.1", "expected", f"unknown relation {expected!r}")
        # Invocation binding (§12.3 K13): scheduled subjects bind through the root.
        inv = claim.get("invocation")
        root = (self.cert.get("bindings") or {}).get("invocation")
        if scheduled:
            if root is None:
                raise _r("E_REPLAY_INVOCATION", "12.3", "bindings.invocation", "absent for a scheduled subject")
            if inv is not None and inv != root:
                raise _r("E_REPLAY_INVOCATION", "12.3", "invocation", "conflicts with the root binding")
            inv = root
        elif inv is None:
            raise _r("E_REPLAY_INVOCATION", "12.3", "invocation", "a standalone claim must carry its own invocation")
        args, budget = _resolve_invocation(self.obj, inv, subj.ptypes, "invocation")
        subj.bind(args)
        dom = _resolve_domain(subj, claim["domain"])
        # §12.2 K75: a legal claim we cannot finish is unchecked, not rejected.
        bits = _full_bits(subj)
        n = _count(dom)
        table = None
        if isinstance(expected, dict) and "table" in expected:
            table = self.parse_table(expected["table"], subj, n)
        program = None
        if isinstance(expected, dict) and "program" in expected:
            program = self.program_subject(expected["program"], subj, args)
        if scheduled:
            self.schedule_binding(s, inv)
        if expected == "oracle":
            raise Unchecked("the realised oracle requires the §11.1 expansion of its full §5.2 construction; "
                            "the replay does not implement that library", "11.1")
        if n > self.max_elements:
            raise Unchecked(f"host limit: {n} elements exceeds the replay bound {self.max_elements}", "12.2")
        for i, (bundles, borrowed, regions) in enumerate(_elements(subj, dom)):
            before = (bundles, borrowed, regions)
            try:
                after = subj.run(budget, bundles, borrowed, regions)
            except Reject as e:
                e.diag.setdefault("witness", {"element": i, "before": _tuple_str(before)})
                raise
            if table is not None:
                row_b, row_a = table[i]
                if row_b != before:
                    raise _r("E_REPLAY_TABLE", "12.1", f"expected.table[{i}].before",
                             "row is not the domain element at this enumeration position",
                             {"element": i, "domain": _tuple_str(before), "row": _tuple_str(row_b)})
                want = row_a
            else:
                want = program.run(budget, bundles, borrowed, regions)
            if tuple(after) != tuple(want):
                raise _r("E_REPLAY_MISMATCH", "12.1", f"element {i}", "full tuple differs from the expected relation",
                         {"element": i, "before": _tuple_str(before), "replayed": _tuple_str(after),
                          "expected": _tuple_str(want)})
        if scheduled:
            # Reference agreement alone is not a claim about the realisation (K13).
            self.establish_realised(name, c, subj)
        frac = Fraction(n, 1 << bits)
        outcome = "proof" if _covers_d(subj, dom) else "evidence"
        if outcome == "evidence" and label == "proof":
            raise _r("E_REPLAY_COVERAGE_LABEL", "12.2", "label",
                     f"requested proof, but the completed check covers {_frac(frac)} of D")
        return {"outcome": outcome, "elements": n, "covered_fraction": _frac(frac)}

    def parse_table(self, rows, subj, n):
        _list(rows, "expected.table", "12.1")
        if len(rows) != n:
            raise _r("E_REPLAY_TABLE", "12.1", "expected.table", f"{len(rows)} rows for {n} domain elements")
        out = []
        for i, row in enumerate(rows):
            p = f"expected.table[{i}]"
            _closed(row, ["before", "after"], [], p, "12.1")
            out.append((_parse_tuple(row["before"], subj, p + ".before"), _parse_tuple(row["after"], subj, p + ".after")))
        return out

    def program_subject(self, h, subj, args):
        if not isinstance(h, str) or not HEX64.match(h):
            raise _r("E_REPLAY_EXPECTED", "12.1", "expected.program", f"{h!r} is not 64-hex")
        name = self.by_closure.get(h)
        if name is None:
            raise _r("E_REPLAY_EXPECTED", "12.1", "expected.program", "names no closure linked into this object")
        other = _Subject(self.obj, name, subj.inherited)
        if other.ptypes == subj.ptypes:
            other.bind(args)
        problems = []
        if (other.kind == 'oracle') != (subj.kind == 'oracle') or other.roles != subj.roles:
            problems.append(f"roles {other.roles} vs {subj.roles}")
        if other.widths != subj.widths:
            problems.append(f"bundle widths {other.widths} vs {subj.widths}")
        if other.shapes != subj.shapes or [r for _n, _t, r in other.mem] != [r for _n, _t, r in subj.mem]:
            problems.append("region shapes or roles differ")
        if other.ptypes != subj.ptypes:
            problems.append(f"parameter types {other.ptypes} vs {subj.ptypes}")
        elif other.borrow_widths != subj.borrow_widths:
            problems.append(f"borrowed widths {other.borrow_widths} vs {subj.borrow_widths}")
        if problems:
            raise _r("E_REPLAY_EXPECTED", "12.1 K73", "expected.program", "signature incompatible: " + "; ".join(problems))
        return other

    def schedule_binding(self, s, inv):
        if self.schedule is None:
            raise Unchecked("no schedule document embedded or supplied", "12.1")
        got = H.sha256hex(jcs.canonical(self.schedule))
        if s["schedule_hash"] != got:
            raise _r("E_REPLAY_BINDING", "9.2", "subject.schedule_hash", f"schedule document hashes to {got}")
        if self.schedule.get("bindings", {}).get("closure_hash") != s["closure_hash"]:
            raise Unchecked("subject is a nested invocation context of the root schedule; context "
                            "enumeration is not replayed", "12.1")
        realised.check_bindings(self.schedule, s, self.closure_of_name, H.params_hash(inv["args"]), inv["budget"])

    def establish_realised(self, name, c, subj):
        if self.target is None:
            raise Unchecked("no target document supplied, so gate exactness cannot be established", "8.3")
        if subj.kind == 'oracle':
            raise Unchecked("the realised oracle requires the §11.1 expansion; not replayed", "11.1")
        realised.establish_exact(self.obj, self.closure_of_name, self.schedule, self.target, name, c)

    # ---- K4 inverse pair
    def inverse_pair(self, rec, index, claim, name, s):
        if s["inherited_controls"] != 0:
            raise _r("E_REPLAY_INVERSE", "12.1", "subject.inherited_controls", "an inverse-pair subject is the bare oracle g")
        if "invocation" in claim:
            raise _r("E_REPLAY_INVERSE", "12.3", "invocation", "K4 claims bind the entry's budget and quantified args, not an invocation")
        vf = rec.get("verification")
        entries = (vf or {}).get("inverse_pairs") if isinstance(vf, dict) else None
        if not isinstance(entries, list):
            raise _r("E_REPLAY_INVERSE", "12.3", "verification.inverse_pairs", "absent")
        mine = [e for e in entries if isinstance(e, dict) and e.get("claim") == index]
        if not mine:
            raise _r("E_REPLAY_INVERSE", "12.1", "verification.inverse_pairs",
                     "an inverse_pair claim must be the claim of an inverse_pairs entry")
        total = 0
        for k, e in enumerate(mine):
            total += self.one_pair(e, claim, name, f"verification.inverse_pairs[{k}]")
        return {"outcome": "proof", "elements": total // len(mine)}

    def one_pair(self, entry, claim, name, path):
        _closed(entry, ["g", "ginv", "claim", "budget"], ["arg_tokens"], path, "12.3")
        g, ginv = self.by_closure.get(entry["g"]), self.by_closure.get(entry["ginv"])
        if g is None or ginv is None:
            raise _r("E_REPLAY_INVERSE", "12.3", path, "g or ginv names no closure of this object")
        if g != name:
            raise _r("E_REPLAY_INVERSE", "12.1", "subject.closure_hash", f"subject @{name}, the entry's g is @{g}")
        budget = _list(entry["budget"], path + ".budget", "12.3")
        if len(budget) != 4:
            raise _r("E_REPLAY_INVERSE", "12.3", path + ".budget", "all four components are required")
        budget = tuple(_nat(x, path + ".budget", "12.3") for x in budget)
        _validate_budget(self.obj, budget, path + ".budget")
        sg, si = _Subject(self.obj, g, 0), _Subject(self.obj, ginv, 0)
        if sg.kind != 'oracle' or si.kind != 'oracle':
            raise _r("E_REPLAY_INVERSE", "4.5", path, "g and ginv must be oracles")
        gins = [type_width(t, self.obj.ptr_width) for _n, t, _s in sg.fn.groups[0]]
        iins = [type_width(t, self.obj.ptr_width) for _n, t, _s in si.fn.groups[0]]
        if gins != iins or not gins or sg.widths[len(gins)] != gins[0] or si.widths[len(iins)] != gins[0] \
                or sg.widths[len(gins) + 1] != si.widths[len(iins) + 1] or sg.shapes != si.shapes or sg.ptypes != si.ptypes:
            raise _r("E_REPLAY_INVERSE", "4.5", path, "g and ginv signatures are not an inverse-pair shape")
        sg.borrow_widths = []
        dom = _resolve_domain(sg, claim["domain"], inverse_pair=True)
        if claim["domain"]["borrowed"]:
            raise _r("E_REPLAY_INVERSE", "12.1", "domain.borrowed", "an oracle pair has no borrowed components")
        want = {("bundle", i) for i in range(len(gins))}
        for r, (A, _W) in enumerate(sg.shapes):
            want |= {("cell", r, i) for i in range(1 << A)}
        got = {c[0] for c in dom.comps}
        full = got == want
        for ident, lo, hi in dom.comps:
            w = gins[ident[1]] if ident[0] == "bundle" else sg.shapes[ident[1]][1]
            full = full and lo == 0 and hi == (1 << w) - 1
        if not full:
            raise _r("E_REPLAY_INVERSE", "12.1", "domain", "bundles and regions must be the full domain of g's in and mem slots")
        tokens = entry.get("arg_tokens", {})
        if not isinstance(tokens, dict):
            raise _r("E_REPLAY_SCHEMA", "12.3", path + ".arg_tokens", "must be an object")
        params = sg.fn.groups[4]
        token_names = {n for n, t, _s in params if t == 'token'}
        if set(tokens) != token_names:
            raise _r("E_REPLAY_INVERSE", "12.3", path + ".arg_tokens",
                     f"binds {sorted(tokens)}, the token arguments are {sorted(token_names)}")
        fixed = {}
        for tn, tup in tokens.items():
            _list(tup, f"{path}.arg_tokens.{tn}", "12.3")
            if len(tup) != 4:
                raise _r("E_REPLAY_INVERSE", "12.3", f"{path}.arg_tokens.{tn}", "a token tuple has four components")
            fixed[tn] = tuple(_nat(x, f"{path}.arg_tokens.{tn}", "12.3") for x in tup)
            _validate_budget(self.obj, fixed[tn], f"{path}.arg_tokens.{tn}")
        # args domain: one range per classical non-token argument
        arg_ranges, seen = [], set()
        by_name = {n: (i, t) for i, (n, t, _s) in enumerate(params) if t != 'token'}
        for j, e in enumerate(_list(claim["domain"].get("args", []), "domain.args", "12.2")):
            p = f"domain.args[{j}]"
            if not isinstance(e, dict) or not isinstance(e.get("name"), str) or e["name"] not in by_name:
                raise _r("E_REPLAY_DOMAIN", "12.2", p, "not a classical non-token argument of g")
            i, t = by_name[e["name"]]
            w, lo, hi = _range_entry(e, p, type_width(t, self.obj.ptr_width))
            if i in seen or lo > hi or hi >= 1 << w:
                raise _r("E_REPLAY_DOMAIN", "12.2", p, "duplicate or invalid argument range")
            seen.add(i)
            arg_ranges.append((i, lo, hi))
        if seen != {i for i, _t in by_name.values()}:
            raise _r("E_REPLAY_INVERSE", "12.3", "domain.args", "every classical non-token argument needs a range; "
                     "missing values have no implicit zero")
        n = _count(dom)
        for _i, lo, hi in arg_ranges:
            n *= hi - lo + 1
        if n > self.max_elements:
            raise Unchecked(f"host limit: {n} elements exceeds the replay bound {self.max_elements}", "12.2")
        tgt_mask = (1 << gins[0]) - 1
        memo = {}

        def outcome(fn, ins, regs, args):
            key = (fn, tuple(ins), tuple(map(tuple, regs)), tuple(args))
            hit = memo.get(key)
            if hit is None:
                hit = memo[key] = oracle_payload(self.obj, fn, ins, regs, args, budget)
            return hit
        arg_product = itertools.product(*[range(lo, hi + 1) for _i, lo, hi in arg_ranges])
        element = -1
        for avals in arg_product:
            args = [None] * len(params)
            for (i, _l, _h), v in zip(arg_ranges, avals):
                args[i] = v
            for nm, _t, _s in params:
                if nm in fixed:
                    args[[p[0] for p in params].index(nm)] = fixed[nm]
            for bundles, _b, regions in _elements(_PairShape(sg, gins), dom):
                element += 1
                x, rest = bundles[0], bundles[1:]
                for first, second, tag in ((g, ginv, "ginv(g(x))"), (ginv, g, "g(ginv(x))")):
                    y, cy = outcome(first, [x] + rest, regions, args)
                    if cy:
                        raise _r("E_REPLAY_INVERSE", "4.5; 12.1", f"element {element}", f"@{first} faults with code {cy}",
                                 {"x": str(x), "ins": [str(v) for v in rest], "args": [str(a) for a in avals]})
                    z, cz = outcome(second, [y & tgt_mask] + rest, regions, args)
                    if cz:
                        raise _r("E_REPLAY_INVERSE", "4.5; 12.1", f"element {element}", f"@{second} faults with code {cz}",
                                 {"x": str(x), "y": str(y), "ins": [str(v) for v in rest]})
                    if z != x:
                        raise _r("E_REPLAY_INVERSE", "4.5; 12.1", f"element {element}", f"{tag} != x",
                                 {"x": str(x), "y": str(y), "z": str(z), "ins": [str(v) for v in rest]})
        return n


class _PairShape:
    """Tuple layout of an inverse-pair element: only g's in slots and mem cells."""

    def __init__(self, sg, gins):
        self.inherited = 0
        self.widths = list(gins)
        self.borrow_widths = []
        self.shapes = sg.shapes


def _frac(f):
    return "1" if f == 1 else f"{f.numerator}/{f.denominator}"


def _diag(code, section, path, message):
    return {"code": code, "section": section, "path": path, "message": message}


def _final(forced, errors, regions, checker, reason=None):
    outcomes = [c.get("outcome") for r in regions.values() for c in r["claims"]]
    region_errors = [e for r in regions.values() for e in r["errors"]]
    if forced:
        overall = forced
    elif errors or region_errors or "reject" in outcomes:
        overall = "reject"
    elif "unchecked" in outcomes:
        overall = "unchecked"
    else:
        overall = "replayed"
    v = {"overall": overall, "errors": errors, "regions": regions,
         "unchecked_scope": SCOPE_NOTES if not forced else [],
         "summary": {k: outcomes.count(k) for k in ("proof", "evidence", "reject", "unchecked")}}
    if reason:
        v["reason"] = reason
    return v


def check_certificate(cert, obj, object_text, target=None, schedule=None, max_elements=DEFAULT_MAX_ELEMENTS):
    return Checker(cert, obj, object_text, target, schedule, max_elements).run()
