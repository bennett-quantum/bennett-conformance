"""Parser for the §16 canonical textual object form (subset the replay needs)."""
import json, re

SECTION_NAMES = ["symbols", "imports", "functions", "data", "devices",
                 "layout", "schedule", "certificate", "coverage"]
NAME_RE = re.compile(r'^[A-Za-z_][A-Za-z0-9_.]*$')


class Instr:
    __slots__ = ("lhs", "op", "operands", "nested", "raw")

    def __init__(self, lhs, op, operands, nested, raw):
        self.lhs, self.op, self.operands, self.nested, self.raw = lhs, op, operands, nested, raw


class Function:
    __slots__ = ("kind", "name", "groups", "regs", "bounds", "blocks", "raw_lines")

    def __init__(self, kind, name, groups):
        self.kind, self.name, self.groups = kind, name, groups
        self.regs, self.bounds, self.blocks, self.raw_lines = [], [], [], []


class Object:
    __slots__ = ("text", "header", "isa", "ptr_width", "symbols", "imports",
                 "functions", "data", "layout", "devices", "schedule",
                 "certificate", "coverage", "symbol_by_name")


def tokenise(s):
    toks, i, n = [], 0, len(s)
    while i < n:
        if s[i] == ' ':
            i += 1
            continue
        j, depth, instr = i, 0, False
        while j < n:
            c = s[j]
            if instr:
                if c == '\\':
                    j += 1
                elif c == '"':
                    instr = False
            elif c == '"':
                instr = True
            elif c in '([':
                depth += 1
            elif c in ')]':
                depth -= 1
            elif c == ' ' and depth == 0:
                break
            j += 1
        toks.append(s[i:j])
        i = j
    return toks


def parse_list(tok):
    if not (tok.startswith('(') and tok.endswith(')')):
        raise ValueError(f"expected list operand, got {tok!r}")
    inner = tok[1:-1].strip()
    return [] if not inner else [p.strip() for p in inner.split(',')]


def _split_groups(rest):
    groups, i, n = [], 0, len(rest)
    while i < n and len(groups) < 5:
        if rest[i] == ' ':
            i += 1
            continue
        assert rest[i] == '(', rest
        depth, j = 0, i
        while j < n:
            if rest[j] == '(':
                depth += 1
            elif rest[j] == ')':
                depth -= 1
                if depth == 0:
                    break
            j += 1
        groups.append(_parse_params(rest[i + 1:j]))
        i = j + 1
    return groups


def _parse_params(inner):
    inner = inner.strip()
    if not inner:
        return []
    out = []
    for part in inner.split(', '):
        bits = part.split(' ')
        name, typ = bits[0].split(':', 1)
        out.append((name, typ, bits[1] if len(bits) > 1 else None))
    return out


def type_width(typ, ptr_width):
    if typ == 'wire':
        return 1
    m = re.match(r'^(?:wire|reg)\[(\d+)\]$', typ)
    if m:
        return int(m.group(1))
    m = re.match(r'^i(\d+)$', typ)
    if m:
        return int(m.group(1))
    if typ == 'ptr':
        return ptr_width
    if typ == 'f64':
        return 64
    raise ValueError(f"no bit width for type {typ!r}")


def region_shape(typ):
    m = re.match(r'^region\[(\d+),(\d+)\]$', typ)
    if not m:
        raise ValueError(f"not a region type {typ!r}")
    return int(m.group(1)), int(m.group(2))


FN_PREFIXES = (("fn cc @", "cc"), ("fn r @", "r"), ("oracle @", "oracle"),
               ("perm_action @", "perm_action"), ("action @", "action"),
               ("lproc @", "lproc"))


def _parse_instrs(lines, idx, closers):
    instrs = []
    while idx < len(lines):
        line = lines[idx]
        if line in closers:
            return instrs, idx, line
        nested = []
        raw = line
        idx += 1
        body = line
        if body.endswith(' {'):
            body = body[:-2]
            while True:
                sub, idx, closer = _parse_instrs(lines, idx, ('}', '} {'))
                nested.append(sub)
                idx += 1
                if closer == '}':
                    break
        lhs = None
        m = re.match(r'^(\([A-Za-z0-9_., ]*\)|[A-Za-z_][A-Za-z0-9_.]*) = (.+)$', body)
        if m:
            lhs, body = m.group(1), m.group(2)
        toks = tokenise(body)
        instrs.append(Instr(lhs, toks[0], toks[1:], nested, raw))
    return instrs, idx, None


def parse_object(text):
    obj = Object()
    obj.text = text
    lines = text.split('\n')
    if lines and lines[-1] == '':
        lines.pop()
    header = lines[0]
    assert header.startswith('bennett-object '), "bad object header"
    obj.header = dict(p.split('=', 1) for p in header.split(' ')[1:])
    obj.isa = obj.header['isa']
    obj.ptr_width = int(obj.header['ptr_width'])
    obj.symbols, obj.imports, obj.functions, obj.data = [], {}, {}, {}
    obj.layout = obj.devices = obj.schedule = obj.certificate = obj.coverage = None
    sections = {}
    cur, start = None, None
    for i, line in enumerate(lines):
        if line.startswith('section '):
            if cur is not None:
                sections[cur] = lines[start:i]
            cur, start = line.split(' ', 1)[1], i + 1
    if cur is not None:
        sections[cur] = lines[start:]
    for name, body in sections.items():
        body = [l for l in body if l != '']
        if name == 'symbols':
            raw = json.loads(body[0])
            obj.symbols = [raw[k] for k in sorted(raw.keys(), key=int)]
        elif name == 'imports':
            obj.imports = json.loads(body[0])
        elif name in ('devices', 'layout', 'schedule', 'certificate', 'coverage'):
            setattr(obj, name, json.loads(body[0]))
        elif name == 'data':
            for l in body:
                nm, _, hx = l.partition(' ')
                obj.data[nm] = hx
        elif name == 'functions':
            i = 0
            while i < len(body):
                line = body[i]
                kind = next(k for p, k in FN_PREFIXES if line.startswith(p))
                at = line.index('@')
                nm = re.match(r'[A-Za-z_][A-Za-z0-9_.]*', line[at + 1:]).group(0)
                rest = line[at + 1 + len(nm):]
                fn = Function(kind, nm, _split_groups(rest))
                fn.raw_lines.append(line)
                i += 1
                if i < len(body) and body[i].startswith('regs '):
                    rl = body[i]
                    fn.raw_lines.append(rl)
                    inner = rl[rl.index('{') + 1:rl.index('}')].strip()
                    if inner:
                        for part in inner.split(', '):
                            n2, t2 = part.split(':', 1)
                            fn.regs.append((n2, t2, None))
                    bm = re.search(r'bounds \{([^}]*)\}', rl)
                    if bm and bm.group(1).strip():
                        fn.bounds = [int(x) for x in bm.group(1).split(', ')]
                    i += 1
                while i < len(body) and body[i].startswith('^'):
                    bl = body[i]
                    fn.raw_lines.append(bl)
                    bname = bl[1:bl.index('(')]
                    bparams = _parse_params(bl[bl.index('(') + 1:bl.rindex(')')])
                    i += 1
                    sub = []
                    depth = 0
                    j = i
                    while j < len(body):
                        l2 = body[j]
                        if depth == 0 and (l2.startswith('^') or
                                            any(l2.startswith(p) for p, _ in FN_PREFIXES)):
                            break
                        if l2 == '}':
                            depth -= 1
                        elif l2 != '} {' and l2.endswith(' {'):
                            depth += 1
                        sub.append(l2)
                        j += 1
                    instrs, _, _ = _parse_instrs(sub, 0, ())
                    fn.raw_lines.extend(sub)
                    fn.blocks.append((bname, bparams, instrs))
                    i = j
                obj.functions[nm] = fn
    obj.symbol_by_name = {s['name']: s for s in obj.symbols}
    return obj
