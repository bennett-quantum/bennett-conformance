"""ISA §9.2 identity, implemented from the spec text independently of every executor.

Used to keep generated objects and shrunk witnesses well-formed. Judged in tests/test_identity.py
against spec example 1, every published object's stored hashes and the neutral suite's closures.
"""
import hashlib
import re

from objfile import jcs

REF = re.compile(r"@([A-Za-z_][A-Za-z0-9_.]*)")
EFFECT = re.compile(r"^effect ([A-Za-z_][A-Za-z0-9_]*)\.")
HOSTCALL = re.compile(r"^(?:\S+ = )?hostcall (\S+) ")
LIFT_ADDR = re.compile(r"^\S+ = lift\.addr @([A-Za-z_][A-Za-z0-9_.]*)$")


class IdentityError(ValueError):
    pass


def sha(data):
    return hashlib.sha256(data.encode("utf-8")).hexdigest()


def _own_regions(o, name):
    lay = o.layout() or {}
    return set((lay.get("interface_regions") or {}).get(name, {}))


def _references(o, f, defined):
    """Symbol references of a function in order of first reference (header's own name excluded)."""
    refs = []
    head = f.lines[0]
    rest = head.split("@%s " % f.name, 1)
    scan = [rest[1] if len(rest) == 2 else head] + f.lines[1:]
    regions = _region_names(f)
    for line in scan:
        for m in REF.finditer(line):
            n = m.group(1)
            if _is_region_token(line, n, regions, defined):
                continue
            if n not in refs:
                refs.append(n)
    return refs


def _region_names(f):
    """Region parameters of an oracle/action header: `@name` naming one is not a symbol reference."""
    return set(re.findall(r"([A-Za-z_][A-Za-z0-9_.]*):region\[", f.lines[0]))


def _is_region_token(line, name, regions, defined):
    return name in regions and LIFT_ADDR.match(line) is not None and name not in defined


def _replace(o, f, table):
    regions = _region_names(f)
    defined = table

    def sub(line):
        def one(m):
            n = m.group(1)
            if _is_region_token(line, n, regions, defined):
                return m.group(0)
            return "@" + table[n]
        return REF.sub(one, line)

    head = f.lines[0]
    prefix, suffix = head.split("@%s " % f.name, 1)
    lines = ["%s@%s %s" % (prefix, f.name, sub(suffix))]
    lines.extend(sub(x) for x in f.lines[1:])
    return "\n".join(lines)


def _layout_leaves(o, f):
    """(path components, value) for the symbol's closure-set layout leaves, per §9.2."""
    lay = o.layout()
    if lay is None:
        return []
    kind = f.kind
    leaves = []
    if kind in ("action", "perm_action"):
        return []
    if kind == "lproc":
        names = []
        for line in f.lines[1:]:
            m = LIFT_ADDR.match(line)
            if m:
                names.append(m.group(1))
        for g, v in (lay.get("globals") or {}).items():
            if g in names:
                leaves.append((["globals", g], v))
        return leaves
    for key, value in lay.items():
        if key == "globals":
            leaves.extend((["globals", g], v) for g, v in value.items())
        elif key == "interface_regions":
            for orc, regs in value.items():
                if kind == "oracle" and orc != f.name:
                    continue
                leaves.extend((["interface_regions", orc, r], v) for r, v in regs.items())
        else:
            leaves.append(([key], value))
    return leaves


def _leaf_lines(o, leaves):
    if o.isa == "0.8":
        rows = sorted(((".".join(p)).encode("utf-8"), ".".join(p), v) for p, v in leaves)
    else:
        rows = sorted((jcs(p).encode("utf-8"), jcs(p), v) for p, v in leaves)
    return ["layout %s %s" % (label, jcs(v)) for _, label, v in rows]


def _data_lines(o, leaves):
    names = set()
    for _path, v in leaves:
        if isinstance(v, dict) and "image" in v:
            names.add(v["image"])
        elif _path == ["block_image"]:
            names.add(v)
    table = dict(o.data)
    missing = names - set(table)
    if missing:
        raise IdentityError("layout names undefined data %s" % sorted(missing))
    return ["data %s %s" % (n, table[n]) for n in sorted(names, key=lambda s: s.encode("utf-8"))]


def _device_lines(o, f):
    dev = o.sections.get("devices") or {}
    models, services = dev.get("models") or {}, dev.get("services") or {}
    used = {}
    for line in f.lines[1:]:
        m = EFFECT.match(line)
        if m:
            if m.group(1) not in models:
                continue  # an undeclared device is the validator's to reject (FAULT_EFFECT_UNMODELLED)
            used[m.group(1)] = models[m.group(1)]
        m = HOSTCALL.match(line)
        if m:
            if m.group(1) not in services:
                continue
            used[m.group(1)] = services[m.group(1)]
    return ["device %s %s" % (n, jcs(used[n])) for n in sorted(used, key=lambda s: s.encode("utf-8"))]


def _sccs(nodes, edges):
    index, low, on, stack, out, counter = {}, {}, set(), [], [], [0]

    def strong(v):
        work = [(v, iter(edges[v]))]
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on.add(v)
        while work:
            node, it = work[-1]
            advanced = False
            for w in it:
                if w not in edges:
                    continue
                if w not in index:
                    index[w] = low[w] = counter[0]
                    counter[0] += 1
                    stack.append(w)
                    on.add(w)
                    work.append((w, iter(edges[w])))
                    advanced = True
                    break
                if w in on:
                    low[node] = min(low[node], index[w])
            if advanced:
                continue
            work.pop()
            if work:
                low[work[-1][0]] = min(low[work[-1][0]], low[node])
            if low[node] == index[node]:
                comp = []
                while True:
                    w = stack.pop()
                    on.discard(w)
                    comp.append(w)
                    if w == node:
                        break
                out.append(comp)

    for v in nodes:
        if v not in index:
            strong(v)
    return out  # reverse topological: dependencies first


def _compute(o, flavour):
    syms = o.symbol_table()
    funcs = {f.name: f for f in o.functions}
    data = dict(o.data)
    imports = o.sections.get("imports") or {}
    defined = set(syms) | set(imports)
    edges = {}
    for name, sym in syms.items():
        if sym["kind"] == "data":
            if name not in data:
                raise IdentityError("data symbol %s has no data line" % name)
            edges[name] = []
        else:
            if name not in funcs:
                raise IdentityError("symbol %s has no function" % name)
            edges[name] = _references(o, funcs[name], defined)
    extern = {}
    for name, imp in imports.items():
        if flavour == "interface":
            extern[name] = sha(name + "\x00" + jcs(imp["signature"]))
        else:
            extern[name] = imp.get("pinned_closure_hash")
    result = {}

    def envelope(name, table, ext):
        sym = syms[name]
        parts = ["isa=" + o.isa, "ptr_width=%d" % o.ptr_width, jcs(sym["signature"])]
        if sym["kind"] == "data":
            parts.append("%s %s" % (name, data[name]))
            return "".join(p + "\n" for p in parts)
        f = funcs[name]
        parts.append(_replace(o, f, table))
        leaves = _layout_leaves(o, f)
        parts.extend(_data_lines(o, leaves))
        parts.extend(_leaf_lines(o, leaves))
        if f.kind in ("cc", "r"):
            parts.extend(_device_lines(o, f))
        parts.extend(ext)
        return "".join(p + "\n" for p in parts)

    for comp in _sccs(list(edges), edges):
        members = sorted(comp, key=lambda s: s.encode("utf-8"))
        cyclic = len(members) > 1 or members[0] in edges[members[0]]
        table = {}
        unknown = False
        for name in edges:
            pass
        for m in members:
            for r in edges[m]:
                if r in members:
                    continue
                h = result.get(r, extern.get(r))
                if r not in result and r not in extern:
                    raise IdentityError("@%s references unknown symbol @%s" % (m, r))
                if h is None:
                    unknown = True
                table[r] = h
        if unknown:
            for m in members:
                result[m] = None
            continue
        if cyclic:
            for i, m in enumerate(members):
                table[m] = "$%d" % i
            envs = [envelope(m, table, [table[r] for r in edges[m] if r not in members]) for m in members]
            scc = sha("".join(envs))
            for i, m in enumerate(members):
                result[m] = sha("%s:%d" % (scc, i))
        else:
            m = members[0]
            result[m] = sha(envelope(m, table, [table[r] for r in edges[m]]))
    return result


def hashes(o):
    interface, closure = _compute(o, "interface"), _compute(o, "closure")
    return {n: {"interface": interface[n], "closure": closure[n]} for n in interface}


def stored_mismatches(o):
    got = hashes(o)
    out = []
    for name, sym in o.symbol_table().items():
        if sym["interface_hash"] != got[name]["interface"]:
            out.append("%s: stored %s computed %s" % (name, sym["interface_hash"], got[name]["interface"]))
    return out


def rehash(o):
    """Rewrite every stored interface_hash from §9.2; the symbols section is re-emitted as JCS."""
    got = hashes(o)
    symbols = o.symbols()
    for sym in symbols.values():
        sym["interface_hash"] = got[sym["name"]]["interface"]
    o.set_section("symbols", symbols)
    return o
