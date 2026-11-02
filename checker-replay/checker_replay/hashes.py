"""§9.2 identity: object hash, interface/closure hashes, params hash."""
import hashlib, re
from . import jcs

REF_RE = re.compile(r'@([A-Za-z_][A-Za-z0-9_.]*)')
DELETED = ("schedule", "certificate", "coverage")


def sha256hex(b):
    return hashlib.sha256(b).hexdigest()


def params_hash(args):
    return sha256hex(jcs.canonical(args))


def object_hash(text):
    lines = text.split('\n')
    ends_nl = bool(lines and lines[-1] == '')
    if ends_nl:
        lines.pop()
    keep, dropping = [], False
    for line in lines:
        if line.startswith('section '):
            name = line.split(' ', 1)[1]
            dropping = name in DELETED
            if dropping:
                if keep and keep[-1] == '':
                    keep.pop()
                continue
        if not dropping:
            keep.append(line)
    out = '\n'.join(keep) + ('\n' if ends_nl else '')
    return sha256hex(out.encode())


def _layout_leaves(layout):
    if not layout:
        return []
    leaves = []
    for key, val in layout.items():
        if key == 'globals':
            for gname, g in val.items():
                leaves.append((["globals", gname], g))
        elif key == 'interface_regions':
            for oname, regs in val.items():
                for rname, r in regs.items():
                    leaves.append((["interface_regions", oname, rname], r))
        else:
            leaves.append(([key], val))
    return leaves


def _images_of(leaves):
    imgs = []
    for path, val in leaves:
        if isinstance(val, dict) and 'image' in val:
            imgs.append(val['image'])
        if path == ['block_image']:
            imgs.append(val)
    return imgs


def _references(obj, fn):
    names, seen = [], set()
    header_done = False
    for line in fn.raw_lines:
        body = line
        if not header_done:
            body = body.replace(f"@{fn.name}", "", 1)
            header_done = True
        for m in REF_RE.finditer(body):
            nm = m.group(1)
            if (nm in obj.symbol_by_name or nm in obj.imports) and nm not in seen:
                seen.add(nm)
                names.append(nm)
    return names


def _closure_set(obj, sym, fn):
    kind = sym['kind']
    leaves = _layout_leaves(obj.layout)
    if kind == 'oracle':
        sel = [(p, v) for p, v in leaves
               if not (len(p) >= 2 and p[0] == 'interface_regions' and p[1] != sym['name'])]
        return sel, sorted(set(_images_of(sel)))
    if kind == 'lproc':
        globals_named = set()
        for line in fn.raw_lines:
            if 'lift.addr' in line:
                globals_named.update(REF_RE.findall(line))
        sel = [(p, v) for p, v in leaves
               if len(p) == 2 and p[0] == 'globals' and p[1] in globals_named]
        return sel, sorted(set(_images_of(sel)))
    if kind in ('action', 'perm_action'):
        return [], []
    if kind in ('cc', 'r'):
        return leaves, sorted(set(_images_of(leaves)))
    return [], []


def _device_lines(obj, sym, fn):
    if sym['kind'] not in ('cc', 'r') or not obj.devices:
        return []
    text = '\n'.join(fn.raw_lines)
    out = []
    for name, entry in sorted((obj.devices.get('models') or {}).items()):
        if re.search(rf'\beffect {re.escape(name)}\.', text):
            out.append(f"device {name} {jcs.canonical(entry).decode()}")
    for name, entry in sorted((obj.devices.get('services') or {}).items()):
        if re.search(rf'\bhostcall "?{re.escape(name)}"?(?: |$)', text, re.M):
            out.append(f"device {name} {jcs.canonical(entry).decode()}")
    return out


def _tarjan(graph):
    index, low, onstack, stack, result, counter = {}, {}, {}, [], [], [0]

    def strong(v):
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        onstack[v] = True
        for w in graph.get(v, []):
            if w not in index:
                strong(w)
                low[v] = min(low[v], low[w])
            elif onstack.get(w):
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop()
                onstack[w] = False
                comp.append(w)
                if w == v:
                    break
            result.append(comp)

    for v in graph:
        if v not in index:
            strong(v)
    return result


def _compute_one(obj, mode, scc_out=None):
    syms = obj.symbol_by_name
    refs = {}
    for name, sym in syms.items():
        fn = obj.functions.get(name)
        refs[name] = [] if fn is None else [r for r in _references(obj, fn) if r in syms]
    sccs = _tarjan({k: refs[k] for k in syms})
    hashes = {}
    for nm, imp in obj.imports.items():
        pinned = imp.get('pinned_closure_hash') if mode == 'closure' else None
        hashes[nm] = pinned or sha256hex(nm.encode() + b'\x00' + jcs.canonical(imp['signature']))

    def envelope(name, scc_members):
        sym = syms[name]
        fn = obj.functions.get(name)
        inside = set(scc_members) if scc_members else set()

        def repl(text, is_header=False):
            def sub(m):
                nm = m.group(1)
                if is_header and nm == name:
                    return m.group(0)
                if nm in inside:
                    return "@$" + str(sorted(inside).index(nm))
                if nm in hashes and (nm in syms or nm in obj.imports):
                    return "@" + hashes[nm]
                return m.group(0)
            return REF_RE.sub(sub, text)

        lines = [f"isa={obj.isa}", f"ptr_width={obj.ptr_width}",
                 jcs.canonical(sym['signature']).decode()]
        if sym['kind'] == 'data':
            lines.append(f"{name} {obj.data.get(name, '')}")
            return ''.join(l + '\n' for l in lines).encode()
        for i, raw in enumerate(fn.raw_lines):
            lines.append(repl(raw, is_header=(i == 0)))
        leaves, images = _closure_set(obj, sym, fn)
        for dn in sorted(images):
            lines.append(f"data {dn} {obj.data.get(dn, '')}")
        leaf_lines = sorted(
            ((jcs.canonical(p), f"layout {jcs.canonical(p).decode()} {jcs.canonical(v).decode()}")
             for p, v in leaves), key=lambda t: t[0])
        lines.extend(l for _, l in leaf_lines)
        lines.extend(_device_lines(obj, sym, fn))
        for r in refs[name]:
            if r not in inside:
                lines.append(hashes[r])
        return ''.join(l + '\n' for l in lines).encode()

    for comp in sccs:
        if len(comp) == 1 and comp[0] not in refs[comp[0]]:
            hashes[comp[0]] = sha256hex(envelope(comp[0], None))
        else:
            members = sorted(comp)
            scc_hash = sha256hex(b''.join(envelope(m, members) for m in members))
            for i, m in enumerate(members):
                hashes[m] = sha256hex(f"{scc_hash}:{i}".encode())
                if scc_out is not None:
                    scc_out[m] = (scc_hash, i)
    return hashes


def index_order_keys(obj):
    """§3.6 placement key of every symbol: (SCC order key, index within the SCC)."""
    sccs = {}
    closure = _compute_one(obj, 'closure', sccs)
    return {name: sccs.get(name, (closure[name], 0)) for name in obj.symbol_by_name}


def compute_hashes(obj):
    iface = _compute_one(obj, 'interface')
    closure = _compute_one(obj, 'closure')
    return {name: (iface.get(name), closure.get(name)) for name in obj.symbol_by_name}
