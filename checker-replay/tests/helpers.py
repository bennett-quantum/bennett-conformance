import copy, json, pathlib
from checker_replay import hashes
from checker_replay.objparse import parse_object
from checker_replay.replay import check_certificate
from tests.spec_vectors import FIXTURES

FIX = FIXTURES / "controlled-action-09"
VERSIONS = {k: "0.9" for k in ("cert", "coverage", "isa", "launch", "object", "sched", "target")}
HEAD = "bennett-object isa=0.9 object=0.9 launch=0.9 sched=0.9 cert=0.9 coverage=0.9 target=0.9 ptr_width=32\n"


def controlled():
    text = (FIX / "controlled.bisa").read_bytes().decode()
    return (text, json.loads((FIX / "certificate.json").read_text()),
            json.loads((FIX / "target.json").read_text()), json.loads((FIX / "schedule.json").read_text()))


def source(fns, data=None, layout=None):
    """Build an object from function text alone, with correct §9.1 signatures and hashes."""
    from checker_replay.objparse import parse_object as P, type_width, region_shape
    text = HEAD + "\nsection symbols\n{}\n\nsection functions\n" + fns
    if data:
        text += "\nsection data\n" + data
    if layout:
        text += "\nsection layout\n" + json.dumps(layout, separators=(",", ":"), sort_keys=True) + "\n"
    obj = P(text)
    syms = {}

    def mem(group):
        out = []
        for _n, t, role in group:
            A, W = region_shape(t)
            out.append({"A": A, "W": W, "role": role})
        return out
    for i, (name, fn) in enumerate(obj.functions.items()):
        g = fn.groups
        w = lambda grp: [type_width(t, 32) for _n, t, _s in grp]
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
    idx = len(syms)
    for dname, hx in sorted((obj.data or {}).items()):
        syms[str(idx)] = {"interface_hash": "0" * 64, "kind": "data", "name": dname,
                          "signature": {"abi": "0.9", "kind": "data", "size": len(hx) // 2, "tier": "-"}}
        idx += 1
    render = lambda s: text.replace("section symbols\n{}", "section symbols\n" + json.dumps(s, separators=(",", ":"), sort_keys=True))
    h = hashes.compute_hashes(P(render(syms)))
    for s in syms.values():
        s["interface_hash"] = h[s["name"]][0]
    return render(syms)


def cert_for(text, root, claims, verification=None):
    obj = parse_object(text)
    h = hashes.compute_hashes(obj)
    ch = h[root][1]
    rec = {"claims": claims}
    if verification is not None:
        rec["verification"] = verification
    return {"versions": dict(VERSIONS),
            "bindings": {"closure_hash": ch, "dialect_mapping_version": "none", "interface_hash": h[root][0],
                         "object_hash": hashes.object_hash(text), "reference_program_hash": h[root][0],
                         "target_hash": "0" * 64},
            "regions": {ch: rec}}, ch, h


def claim_of(text, cert, ch, i=0, **kw):
    v = check_certificate(cert, parse_object(text), text, **kw)
    return v["regions"][ch]["claims"][i]


def tup(bundles, borrowed=(), regions=()):
    return {"bundles": [str(b) for b in bundles], "borrowed": [str(b) for b in borrowed],
            "regions": [[str(c) for c in r] for r in regions]}


def rng(name, width, lo=0, hi=None):
    return {"name": name, "width": width, "lo": lo, "hi": (1 << width) - 1 if hi is None else hi}
