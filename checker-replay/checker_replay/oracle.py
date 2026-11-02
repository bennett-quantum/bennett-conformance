"""§14.1 reference interpreter for oracle bodies and their lproc closure.

Classical per-basis execution of §4.2 rows: one budget step per row; halt by
lift.retval, a fault row or the budget. Activation records follow §3.7, with its
monotonic stack: `sp` is never restored by lift.ret. Fixed binding instances
(§3.7) own their locations; label values are the canonical §3.6 indices.
"""
from .errors import ObjectError, Unchecked
from .objparse import parse_list, type_width, region_shape
from .hashes import index_order_keys

FAULT_BUDGET_STEPS, FAULT_BUDGET_HEAP, FAULT_BUDGET_DEPTH, FAULT_UB_TRAP = 1, 2, 3, 4
FAULT_UB_DIV0, FAULT_UB_SDIV_OVERFLOW, FAULT_UB_MEMFAULT = 5, 6, 7

_BIN = {
    'add': lambda a, b, w: a + b, 'sub': lambda a, b, w: a - b, 'mul': lambda a, b, w: a * b,
    'and': lambda a, b, w: a & b, 'or': lambda a, b, w: a | b, 'xor': lambda a, b, w: a ^ b,
    # §4.1 "shift >= N -> 0" has no sign exception for ashr.
    'shl': lambda a, b, w: a << b if b < w else 0,
    'lshr': lambda a, b, w: a >> b if b < w else 0,
    'ashr': lambda a, b, w: _signed(a, w) >> b if b < w else 0,
}
_CMP = {
    'eq': lambda a, b, sa, sb: a == b, 'ne': lambda a, b, sa, sb: a != b,
    'ult': lambda a, b, sa, sb: a < b, 'ule': lambda a, b, sa, sb: a <= b,
    'ugt': lambda a, b, sa, sb: a > b, 'uge': lambda a, b, sa, sb: a >= b,
    'slt': lambda a, b, sa, sb: sa < sb, 'sle': lambda a, b, sa, sb: sa <= sb,
    'sgt': lambda a, b, sa, sb: sa > sb, 'sge': lambda a, b, sa, sb: sa >= sb,
}


def _mask(v, w):
    return v & ((1 << w) - 1)


def _signed(v, w):
    return v - (1 << w) if v >> (w - 1) else v


class _Halt(Exception):
    def __init__(self, code):
        self.code = code


class _Retval(Exception):
    def __init__(self, value):
        self.value = value


class _Instance:
    """An oracle, or an (lproc, binding key) instance: its locations and fixed bindings."""
    __slots__ = ("fn", "key", "widths", "locs", "pseudo", "tokens", "regions", "labels", "rows")

    def __init__(self, fn, key=()):
        self.fn, self.key = fn, key
        self.widths, self.locs, self.pseudo, self.tokens, self.regions = {}, {}, {}, {}, {}
        self.labels = {b[0]: b[2] for b in fn.blocks}


class _Memory:
    def __init__(self, obj, name, regions, shapes):
        self.layout = obj.layout
        self.cells = {}
        self.tab = []  # [base, size, live, pinned]
        self.region_base = []
        self.heap_used = 0
        if self.layout is None:
            if regions:
                raise ObjectError("7.2", f"@{name}: memoryless object with mem slots")
            return
        for g, ent in (self.layout.get("globals") or {}).items():
            img = bytes.fromhex(obj.data.get(ent["image"], ""))
            for i, b in enumerate(img):
                self.cells[ent["base"] + i] = b
            self.tab.append([ent["base"], ent["size"], 1, 1])
        mapped = (self.layout.get("interface_regions") or {}).get(name, {})
        # §3.7: pre-entries follow layout.interface_regions order.
        order = {n: i for i, (n, _t, _s) in enumerate(obj.functions[name].groups[3])}
        if set(mapped) != set(order):
            raise ObjectError("7.2", f"@{name}: layout maps regions {sorted(mapped)}, header declares {sorted(order)}")
        self.region_base = [None] * len(order)
        for rname, ent in mapped.items():
            pos = order[rname]
            A, W = shapes[pos]
            if W != 8:
                raise ObjectError("7.2", f"@{name}: interface region {rname} has W={W}; W=8 is required")
            self.region_base[pos] = ent["base"]
            self.tab.append([ent["base"], 1 << A, 1, 1])
            for i, c in enumerate(regions[pos]):
                self.cells[ent["base"] + i] = c

    def inrange(self, p, n):
        return any(live and base <= p and p + n <= base + size for base, size, live, _ in self.tab)

    def load(self, a, nb):
        return sum(self.cells.get(a + i, 0) << (8 * i) for i in range(nb))

    def store(self, a, v, nb):
        for i in range(nb):
            self.cells[a + i] = (v >> (8 * i)) & 0xFF

    def region_cells(self, pos, ncells):
        base = self.region_base[pos]
        return [self.cells.get(base + i, 0) for i in range(ncells)]


def _validate_tuple(budget):
    if len(budget) != 4:
        raise ObjectError("5.4", f"budget tuple {budget!r} is not four components")
    return tuple(int(x) for x in budget)


class _Run:
    def __init__(self, obj, name, ins, regions, args, budget):
        self.obj = obj
        self.name = name
        self.fn = obj.functions[name]
        self.B, self.heap_limit, self.stack_limit, self.depth_limit = _validate_tuple(budget)
        shapes = [region_shape(t) for (_n, t, _s) in self.fn.groups[3]]
        self.regions_in = regions
        self.mem = _Memory(obj, name, regions, shapes)
        self.args = list(args)
        self.instances = {}
        self.index = None
        self.root = _Instance(self.fn)
        groups = self.fn.groups
        if len(ins) != len(groups[0]):
            raise ObjectError("5.1", f"@{name}: {len(ins)} input values for {len(groups[0])} in slots")
        for (nm, typ, _s), v in zip(groups[0], ins):
            w = type_width(typ, obj.ptr_width)
            self.root.widths[nm] = w
            self.root.pseudo[nm] = _mask(v, w)
        self._bind_args(self.root, groups[4], list(range(len(groups[4]))))
        for pos, (nm, _t, _s) in enumerate(groups[3]):
            self.root.regions[nm] = pos
        for nm, typ, _s in self.fn.regs:
            self.root.widths[nm] = type_width(typ, obj.ptr_width)
            self.root.locs[nm] = 0
        self.arg_pos = {nm: i for i, (nm, _t, _s) in enumerate(groups[4])}
        self.root_args_positions = {nm: i for i, (nm, _t, _s) in enumerate(groups[4])}
        self.positions = {id(self.root): {nm: i for i, (nm, _t, _s) in enumerate(groups[4])}}

    def _bind_args(self, inst, params, positions):
        for (nm, typ, _s), pos in zip(params, positions):
            v = self.args[pos]
            if typ == 'token':
                inst.tokens[nm] = tuple(int(x) for x in v)
            else:
                w = type_width(typ, self.obj.ptr_width)
                inst.widths[nm] = w
                inst.pseudo[nm] = _mask(_arg_bits(v), w)

    # ---- instances and the §3.6 index space
    def instance(self, name, key):
        ident = (name, key)
        inst = self.instances.get(ident)
        if inst is not None:
            return inst
        fn = self.obj.functions.get(name)
        if fn is None or fn.kind != 'lproc':
            raise Unchecked(f"lift.call target @{name} is not a defined lproc of this object (linking not replayed)", "9.3")
        inst = _Instance(fn, key)
        g = fn.groups
        nt, nc = len(g[2]), len(g[3])
        self._bind_args(inst, g[2], key[:nt])
        self._bind_args(inst, g[3], key[nt:nt + nc])
        for (nm, _t, _s), pos in zip(g[4], key[nt + nc:]):
            inst.regions[nm] = pos
        for nm, typ, _s in list(g[0]) + list(g[1]) + list(fn.regs):
            inst.widths[nm] = type_width(typ, self.obj.ptr_width)
            inst.locs[nm] = 0
        self.positions[id(inst)] = {}
        for (nm, _t, _s), pos in zip(list(g[2]) + list(g[3]), key[:nt + nc]):
            self.positions[id(inst)][nm] = pos
        self.instances[ident] = inst
        return inst

    def call_key(self, inst, callee, toks, clas, mems):
        cg = callee.groups
        if len(toks) != len(cg[2]) or len(clas) != len(cg[3]) or len(mems) != len(cg[4]):
            raise ObjectError("3.7", f"lift.call @{callee.name}: binding lists do not match the callee groups")
        pos = self.positions[id(inst)]
        try:
            key = [pos[t] for t in toks] + [pos[c] for c in clas] + [inst.regions[m] for m in mems]
        except KeyError as e:
            raise ObjectError("3.7", f"lift.call @{callee.name}: {e} is not a binding of the caller") from None
        return tuple(key)

    def _closure(self):
        seen, work = {}, [self.root]
        while work:
            inst = work.pop()
            for _b, _p, instrs in inst.fn.blocks:
                for row in instrs:
                    if row.op != 'lift.call':
                        continue
                    cname = row.operands[0][1:]
                    callee = self.obj.functions.get(cname)
                    if callee is None:
                        raise Unchecked(f"lift.call target @{cname} is not defined in this object", "9.3")
                    toks, clas, mems = _call_lists(row)
                    key = self.call_key(inst, callee, toks, clas, mems)
                    if (cname, key) not in seen:
                        seen[(cname, key)] = self.instance(cname, key)
                        work.append(seen[(cname, key)])
        return seen

    def index_space(self):
        if self.index is not None:
            return self.index
        keys = index_order_keys(self.obj)
        closure = self._closure()
        order = sorted(closure, key=lambda ident: (keys[ident[0]][0], keys[ident[0]][1], ident[1]))
        first = {}
        rows = []
        for inst in [self.root] + [closure[i] for i in order]:
            first[id(inst)] = len(rows)
            for bname, _p, instrs in inst.fn.blocks:
                inst_rows = [(inst, bname, pc) for pc in range(len(instrs))]
                rows.extend(inst_rows)
            inst.rows = (first[id(inst)], len(rows))
        self.index = (first, rows)
        return self.index

    def label(self, inst, bname):
        first, rows = self.index_space()
        lo, hi = inst.rows
        for k in range(lo, hi):
            if rows[k][1] == bname and rows[k][2] == 0:
                return k
        raise ObjectError("3.6", f"label ^{bname} does not name a block of @{inst.fn.name}")

    # ---- operands
    def read(self, inst, tok):
        if tok in inst.locs:
            return inst.locs[tok]
        if tok in inst.pseudo:
            return inst.pseudo[tok]
        raise ObjectError("3.7", f"@{inst.fn.name}: read operand {tok!r} is not a location or pseudo-location")

    def read_len(self, inst, tok):
        return int(tok) if _is_int(tok) else self.read(inst, tok)

    def width(self, inst, tok):
        try:
            return inst.widths[tok]
        except KeyError:
            raise ObjectError("3.3", f"@{inst.fn.name}: operand {tok!r} has no declared width") from None

    def write(self, inst, lhs, v):
        if lhs not in inst.locs:
            raise ObjectError("3.7", f"@{inst.fn.name}: written operand {lhs!r} is not a location")
        inst.locs[lhs] = _mask(v, inst.widths[lhs])

    def need_mem(self, row):
        if self.mem.layout is None:
            raise ObjectError("7.2", f"row {row.raw!r} in a memoryless object")

    def callee_args(self, inst, names, params):
        if len(names) != len(params):
            raise ObjectError("4.2", f"args list {names} does not match callee arity {len(params)}")
        out = []
        for tok, (_n, typ, _s) in zip(names, params):
            if typ == 'token':
                if tok not in inst.tokens:
                    raise ObjectError("4.2", f"{tok!r} is not a token binding of @{inst.fn.name}")
                out.append(inst.tokens[tok])
            else:
                if tok not in inst.pseudo:
                    raise ObjectError("4.2", f"{tok!r} is not a classical binding of @{inst.fn.name}")
                out.append(inst.pseudo[tok])
        return out

    def callee_regions(self, inst, names, params):
        if len(names) != len(params):
            raise ObjectError("4.2", f"mem list {names} does not match callee arity {len(params)}")
        out = []
        for rname, (_n, typ, _s) in zip(names, params):
            A, _W = region_shape(typ)
            if rname not in inst.regions:
                raise ObjectError("4.2", f"{rname!r} is not a mapped region of @{inst.fn.name}")
            out.append(self.mem.region_cells(inst.regions[rname], 1 << A))
        return out

    # ---- execution
    def run(self):
        inst, bname, pc = self.root, self.fn.blocks[0][0], 0
        steps, depth, sp = 0, 0, 0
        stack = []
        try:
            while True:
                if steps >= self.B:
                    return (0, FAULT_BUDGET_STEPS)
                instrs = inst.labels[bname]
                if pc >= len(instrs):
                    raise ObjectError("3.6", f"block ^{bname} of @{inst.fn.name} has no terminator")
                row = instrs[pc]
                steps += 1
                pc += 1
                op, ops, lhs = row.op, row.operands, row.lhs
                if op == 'qjump':
                    bname, pc = _plain_label(ops[0]), 0
                elif op == 'qbranch':
                    bname, pc = _plain_label(ops[1] if self.read(inst, ops[0]) else ops[2]), 0
                elif op == 'qjumpr':
                    a = self.read(inst, ops[0])
                    _first, rows = self.index_space()
                    lo, hi = inst.rows
                    if a < lo or a >= hi:
                        raise _Halt(FAULT_UB_TRAP)
                    _i, bname, pc = rows[a]
                elif op == 'lift.call':
                    cname = ops[0][1:]
                    callee_fn = self.obj.functions.get(cname)
                    if callee_fn is None:
                        raise Unchecked(f"lift.call target @{cname} is not defined in this object", "9.3")
                    self.need_mem(row)
                    toks, clas, mems = _call_lists(row)
                    key = self.call_key(inst, callee_fn, toks, clas, mems)
                    callee = self.instance(cname, key)
                    actuals = parse_list(ops[1])
                    if len(actuals) != len(callee_fn.groups[0]):
                        raise ObjectError("3.7", f"lift.call @{cname}: {len(actuals)} location arguments")
                    r_f = 3 + len(callee.locs)
                    if depth + 1 > self.depth_limit or sp + r_f > self.stack_limit:
                        raise _Halt(FAULT_BUDGET_DEPTH)
                    captured = [self.read(inst, a) for a in actuals]  # C1 reads before any location moves
                    stack.append((inst, bname, pc, lhs, dict(callee.locs)))  # C2/C3
                    for k in callee.locs:
                        callee.locs[k] = 0
                    for (nm, _t, _s), v in zip(callee_fn.groups[0], captured):  # C4
                        callee.locs[nm] = _mask(v, callee.widths[nm])
                    sp += r_f  # C5; monotonic, lift.ret does not restore sp
                    depth += 1
                    inst, bname, pc = callee, callee_fn.blocks[0][0], 0
                elif op == 'lift.ret':
                    if inst is self.root:
                        raise ObjectError("4.2", "lift.ret outside an lproc")
                    if depth == 0:
                        raise _Halt(FAULT_UB_TRAP)
                    v = self.read(inst, ops[0])
                    caller, rb, rpc, w, saved = stack.pop()
                    inst.locs.clear()
                    inst.locs.update(saved)  # R3
                    self.write(caller, w, v)  # R4
                    depth -= 1
                    inst, bname, pc = caller, rb, rpc
                else:
                    self.row(inst, row, op, ops, lhs)
        except _Halt as h:
            return (0, h.code)
        except _Retval as r:
            return (r.value, 0)

    def row(self, inst, row, op, ops, lhs):
        sig_tgt = self.obj.symbol_by_name[self.name]["signature"]["tgt"]
        if op.startswith('lift.const.'):
            self.write(inst, lhs, int(ops[0]))
        elif op == 'lift.mov':
            self.write(inst, lhs, self.read(inst, ops[0]))
        elif op == 'lift.parmov':
            dsts, srcs = parse_list(lhs), parse_list(ops[0])
            if len(dsts) != len(srcs):
                raise ObjectError("4.2", f"lift.parmov arity {row.raw!r}")
            vals = [self.read(inst, s) for s in srcs]
            for d, v in zip(dsts, vals):
                self.write(inst, d, v)
        elif op[5:] in _BIN and op.startswith('lift.'):
            a, b = self.read(inst, ops[0]), self.read(inst, ops[1])
            self.write(inst, lhs, _BIN[op[5:]](a, b, self.width(inst, lhs)))
        elif op in ('lift.udiv', 'lift.urem', 'lift.sdiv', 'lift.srem'):
            a, b = self.read(inst, ops[0]), self.read(inst, ops[1])
            w = self.width(inst, lhs)
            signed = op[5] == 's'
            if b == 0:
                raise _Halt(FAULT_UB_DIV0)
            if signed and a == 1 << (w - 1) and b == (1 << w) - 1:
                raise _Halt(FAULT_UB_SDIV_OVERFLOW)
            if signed:
                sa, sb = _signed(a, w), _signed(b, w)
                q = abs(sa) // abs(sb)
                q = -q if (sa < 0) != (sb < 0) else q
                self.write(inst, lhs, q if op == 'lift.sdiv' else sa - q * sb)
            else:
                self.write(inst, lhs, a // b if op == 'lift.udiv' else a % b)
        elif op.startswith('lift.icmp.'):
            pred = op[len('lift.icmp.'):]
            if pred not in _CMP:
                raise ObjectError("4.2", f"unknown predicate {op}")
            a, b = self.read(inst, ops[0]), self.read(inst, ops[1])
            w = self.width(inst, ops[0])
            self.write(inst, lhs, 1 if _CMP[pred](a, b, _signed(a, w), _signed(b, w)) else 0)
        elif op == 'lift.select':
            self.write(inst, lhs, self.read(inst, ops[1]) if self.read(inst, ops[0]) else self.read(inst, ops[2]))
        elif op.startswith('lift.zext.') or op.startswith('lift.trunc.'):
            self.write(inst, lhs, self.read(inst, ops[0]))
        elif op.startswith('lift.sext.'):
            self.write(inst, lhs, _signed(self.read(inst, ops[0]), self.width(inst, ops[0])))
        elif op == 'lift.label':
            self.write(inst, lhs, self.label(inst, _plain_label(ops[0])))
        elif op == 'lift.addr':
            self.need_mem(row)
            g = ops[0][1:]
            globs = self.mem.layout.get("globals") or {}
            if g in globs:
                self.write(inst, lhs, globs[g]["base"])
            elif inst is self.root and g in inst.regions:
                self.write(inst, lhs, self.mem.region_base[inst.regions[g]])
            else:
                raise ObjectError("4.2", f"lift.addr @{g}: no such global, or region outside its declaring oracle")
        elif op == 'lift.boundscheck':
            self.need_mem(row)
            if not self.mem.inrange(self.read(inst, ops[0]), self.read_len(inst, ops[1])):
                raise _Halt(FAULT_UB_MEMFAULT)
        elif op.startswith('lift.load.i'):
            self.need_mem(row)
            nb = int(op[len('lift.load.i'):]) // 8
            a = self.read(inst, ops[0])
            if not self.mem.inrange(a, nb):
                raise _Halt(FAULT_UB_MEMFAULT)
            self.write(inst, lhs, self.mem.load(a, nb))
        elif op.startswith('lift.store.i'):
            self.need_mem(row)
            nb = int(op[len('lift.store.i'):]) // 8
            a = self.read(inst, ops[0])
            if not self.mem.inrange(a, nb):
                raise _Halt(FAULT_UB_MEMFAULT)
            self.mem.store(a, self.read(inst, ops[1]), nb)
        elif op == 'lift.alloc':
            self.need_mem(row)
            n = self.read_len(inst, ops[0])
            need = max(n, 1)
            if self.mem.heap_used + need > self.heap_limit or len(self.mem.tab) >= 1 << self.mem.layout["A_tab"]:
                raise _Halt(FAULT_BUDGET_HEAP)
            base = self.mem.layout["heap_base"] + self.mem.heap_used
            self.mem.tab.append([base, n, 1, 0])
            self.mem.heap_used += need
            self.write(inst, lhs, base)
        elif op == 'lift.free':
            self.need_mem(row)
            p = self.read(inst, ops[0])
            hit = [e for e in self.mem.tab if e[0] == p and not e[3] and e[2]]
            if not hit:
                raise _Halt(FAULT_UB_MEMFAULT)
            hit[0][2] = 0
        elif op == 'lift.faultif':
            if self.read(inst, ops[0]):
                raise _Halt(int(ops[1]) or FAULT_UB_TRAP)
        elif op == 'fault':
            raise _Halt(int(ops[0]) or FAULT_UB_TRAP)
        elif op == 'lift.retval':
            if inst is not self.root:
                raise ObjectError("4.2", "lift.retval in an lproc")
            raise _Retval(_mask(self.read(inst, ops[0]), sig_tgt))
        elif op in ('lift.oracle_call', 'lift.oracle_uncall'):
            self.nested_oracle(inst, ops)
        elif op == 'lift.perm_call':
            self.perm_call(inst, ops)
        else:
            raise Unchecked(f"unsupported lifted row {op!r} in @{inst.fn.name}", "4.2")

    def nested_oracle(self, inst, ops):
        cname = ops[0][1:]
        cfn = self.obj.functions.get(cname)
        if cfn is None or cfn.kind != 'oracle':
            raise Unchecked(f"lift.oracle_call target @{cname} is not defined in this object", "9.3")
        if ops[1] not in inst.tokens:
            raise ObjectError("4.2", f"token operand {ops[1]!r} is not a token binding of @{inst.fn.name}")
        ins = [self.read(inst, t) for t in parse_list(ops[2])]
        regs = self.callee_regions(inst, parse_list(ops[5]), cfn.groups[3])
        cargs = self.callee_args(inst, parse_list(ops[6]), cfn.groups[4])
        payload, code = oracle_payload(self.obj, cname, ins, regs, cargs, inst.tokens[ops[1]])
        self.write(inst, ops[3], self.read(inst, ops[3]) ^ payload)
        self.write(inst, ops[4], self.read(inst, ops[4]) ^ code)

    def perm_call(self, inst, ops):
        from .action import compile_action
        cname = ops[0][1:]
        cfn = self.obj.functions.get(cname)
        if cfn is None or cfn.kind not in ('perm_action', 'action'):
            raise Unchecked(f"lift.perm_call target @{cname} is not defined in this object", "9.3")
        muts, ins = parse_list(ops[1]), parse_list(ops[2])
        regs = self.callee_regions(inst, parse_list(ops[3]), cfn.groups[2])
        cargs = self.callee_args(inst, parse_list(ops[4]), cfn.groups[3])
        prog = compile_action(self.obj, cname, cargs, 0)
        if prog.borrow_widths:
            raise Unchecked(f"lift.perm_call @{cname} borrows wires; the lifted reference gives them no "
                            "named source (see the K13 borrowed-oracle contract question)", "12.1")
        after, _b, _r = prog.run([self.read(inst, t) for t in muts + ins], [], regs)
        for t, v in zip(muts, after[:len(muts)]):
            self.write(inst, t, v)


def _is_int(tok):
    t = tok[1:] if tok.startswith('-') else tok
    return t.isdigit()


def _plain_label(tok):
    if not tok.startswith('^') or '(' in tok:
        raise ObjectError("3.6", f"lifted label operand {tok!r}")
    return tok[1:]


def _arg_bits(v):
    if isinstance(v, str) and v.startswith('0x'):
        return int(v, 16)
    return int(v)


def _call_lists(row):
    ops = row.operands
    if len(ops) == 2:
        return [], [], []
    if len(ops) != 5:
        raise ObjectError("3.7", f"lift.call form {row.raw!r}: two or five operand lists are required")
    return parse_list(ops[2]), parse_list(ops[3]), parse_list(ops[4])


def oracle_payload(obj, name, ins, regions, args, budget):
    """(payload, code) of F_B. `regions` is a list of cell-value lists in mem-slot order."""
    return _Run(obj, name, ins, regions, args, budget).run()


def oracle_map(obj, name, ins, tgt, fault, regions, args, budget):
    """§5.2 contract on (tgt, fr): each XORed with the F_B outcome."""
    payload, code = oracle_payload(obj, name, ins, regions, args, budget)
    sig = obj.symbol_by_name[name]["signature"]
    return (tgt ^ _mask(payload, sig["tgt"]), fault ^ _mask(code, sig["fault"]))
