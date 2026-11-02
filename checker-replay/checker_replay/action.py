"""§5.3 reference semantics of actions, compiled per invocation to a flat op list.

Control flow in an action is static (§3.5: allocation is unconditional, gates are
predicated), so one invocation compiles to one operation list over integer wire
ids, including the §4.5 qinplace sem and §4.6 expansions. `adj` compiles the callee
and reverses it (§5.3); borrow positions then follow execution order (§12.2).

Permutation-only programs run on one integer basis state. Any h/s/t family gate
switches to a sparse exact simulation over Q(zeta_8), so interference and phase are
exact and a non-+1-basis output is a rejection (§12.1 K13, T3), never rounding.
rx/ry/rz are not representable there and are unchecked, not approximated.
"""
from fractions import Fraction
from .errors import ObjectError, Reject, Unchecked
from .objparse import parse_list, type_width, region_shape
from .oracle import oracle_payload

X, SWAP, CHECK, PHASE, H, ORACLE, MARK = range(7)
PERM_GATES = {'x', 'cx', 'ccx', 'mcx', 'swap'}
PHASE_K = {'s': 2, 'sdg': 6, 't': 1, 'tdg': 7}
ONE = (Fraction(1), Fraction(0), Fraction(0), Fraction(0))


def _mul_w(a, k):
    """a * zeta_8^k for a in the basis 1, w, w^2, w^3 (w^4 = -1)."""
    out = list(a)
    for _ in range(k % 8):
        out = [-out[3], out[0], out[1], out[2]]
    return tuple(out)


def _add(a, b):
    return tuple(x + y for x, y in zip(a, b))


def _scale_h(a):
    """a / sqrt(2), with 1/sqrt(2) = (w - w^3)/2."""
    return tuple(Fraction(1, 2) * (p - q) for p, q in zip(_mul_w(a, 1), _mul_w(a, 3)))


class Program:
    """Flat ops plus the interface layout of one compiled invocation."""

    def __init__(self, name):
        self.name = name
        self.ops = []
        self.nwires = 0
        self.controls = []
        self.bundles = []       # [wire ids], mut then in
        self.nmut = 0
        self.regions = []       # [[wire ids per cell]]
        self.region_roles = []
        self.borrow_wires = []  # final execution order
        self.borrow_widths = []
        self.pure_perm = True
        self.names = {}
        self.region_names = {}

    def fresh(self, n):
        w = list(range(self.nwires, self.nwires + n))
        self.nwires += n
        return w

    def _load(self, wires, v, state):
        for i, w in enumerate(wires):
            if (v >> i) & 1:
                state |= 1 << w
        return state

    @staticmethod
    def _read(wires, state):
        return sum(((state >> w) & 1) << i for i, w in enumerate(wires))

    def run(self, bundles, borrowed, regions):
        """Map (controls+bundles, borrowed, region cells) to the output tuple, or reject."""
        all_bundles = self.controls + self.bundles
        if len(bundles) != len(all_bundles):
            raise ObjectError("12.1", f"@{self.name}: {len(bundles)} bundle values for {len(all_bundles)} components")
        state = 0
        for wires, v in zip(all_bundles, bundles):
            state = self._load(wires, v, state)
        for wires, v in zip(self.borrow_wires, borrowed):
            state = self._load(wires, v, state)
        for cells, vals in zip(self.regions, regions):
            for wires, v in zip(cells, vals):
                state = self._load(wires, v, state)
        final = self._run_perm(state) if self.pure_perm else self._run_exact(state)
        after = ([self._read(w, final) for w in all_bundles],
                 [self._read(w, final) for w in self.borrow_wires],
                 [[self._read(c, final) for c in cells] for cells in self.regions])
        self._check_preserved(bundles, borrowed, regions, after)
        return after

    def _check_preserved(self, bundles, borrowed, regions, after):
        """§5.3/§5.6: inherited controls, `in` bundles and regions, borrowed systems unchanged."""
        first_in = len(self.controls) + self.nmut
        for i, (b, a) in enumerate(zip(bundles, after[0])):
            if (i < len(self.controls) or i >= first_in) and a != b:
                raise Reject("E_REPLAY_INPUT_CHANGED", "5.3", "", f"@{self.name}: read-only bundle {i} changed {b} -> {a}")
        if list(borrowed) != after[1]:
            raise Reject("E_REPLAY_BORROW", "5.6", "", f"@{self.name}: borrowed {list(borrowed)} -> {after[1]}")
        for i, role in enumerate(self.region_roles):
            if role == 'in' and list(regions[i]) != after[2][i]:
                raise Reject("E_REPLAY_INPUT_CHANGED", "5.1", "", f"@{self.name}: `in` region slot {i} changed")

    def _run_perm(self, s):
        for op in self.ops:
            code = op[0]
            if code == X:
                if s & op[1] == op[2]:
                    s ^= op[3]
            elif code == SWAP:
                if s & op[1] == op[2] and ((s >> op[3]) ^ (s >> op[4])) & 1:
                    s ^= (1 << op[3]) | (1 << op[4])
            elif code == CHECK:
                if s & op[1] != op[2]:
                    raise self._dirty(op, s)
            else:
                s = self._oracle(op, s)
        return s

    def _run_exact(self, s0):
        amps = {s0: ONE}
        for op in self.ops:
            code = op[0]
            if code == X:
                amps = {(s ^ op[3] if s & op[1] == op[2] else s): a for s, a in amps.items()}
            elif code == SWAP:
                amps = {(s ^ ((1 << op[3]) | (1 << op[4]))
                         if s & op[1] == op[2] and ((s >> op[3]) ^ (s >> op[4])) & 1 else s): a
                        for s, a in amps.items()}
            elif code == CHECK:
                for s in amps:
                    if s & op[1] != op[2]:
                        raise self._dirty(op, s)
            elif code == PHASE:
                amps = {s: (_mul_w(a, op[3]) if s & op[1] == op[2] else a) for s, a in amps.items()}
            elif code == H:
                out = {}
                for s, a in amps.items():
                    if s & op[1] != op[2]:
                        out[s] = _add(out.get(s, (0, 0, 0, 0)), a)
                        continue
                    h = _scale_h(a)
                    bit = 1 << op[3]
                    s0_, s1_ = s & ~bit, s | bit
                    out[s0_] = _add(out.get(s0_, (0, 0, 0, 0)), h)
                    out[s1_] = _add(out.get(s1_, (0, 0, 0, 0)), h if not s & bit else tuple(-x for x in h))
                amps = {s: a for s, a in out.items() if any(a)}
            else:
                amps = {self._oracle(op, s): a for s, a in amps.items()}
        if len(amps) != 1 or next(iter(amps.values())) != ONE:
            raise Reject("E_REPLAY_NOT_PERMUTATION", "14.3 T3; 12.1 K13", "",
                         f"@{self.name}: output is not one basis state at exact +1 phase "
                         f"({len(amps)} branches); use a numerical/channel claim for a non-permutation")
        return next(iter(amps))

    def _dirty(self, op, s):
        return Reject("E_REPLAY_DIRTY", "14.3 T3; 5.1", "",
                      f"@{self.name}: {op[3]} is not at its required value on this basis configuration")

    def _oracle(self, op, s):
        if s & op[1] != op[2]:
            return s
        spec = op[3]
        ins = [self._read(w, s) for w in spec['ins']]
        regs = [[self._read(c, s) for c in cells] for cells in spec['mem']]
        payload, code = spec['fn'](ins, regs)
        for w, v in ((spec['tgt'], payload), (spec['fr'], code)):
            for i, wire in enumerate(w):
                if (v >> i) & 1:
                    s ^= 1 << wire
        return s

    def run_flat(self, before, order):
        """K78-K80 vector form: `order` names tuple wires, LSB first; returns the same encoding."""
        pos = {}
        for name, wires in self.names.items():
            for i, w in enumerate(wires):
                pos[f"{name}[{i}]"] = w
                if len(wires) == 1:
                    pos[name] = w
        if self.controls:
            pos['control'] = self.controls[0][0]
        if self.borrow_wires:
            pos['borrowed'] = self.borrow_wires[0][0]
        for name, cells in self.region_names.items():
            for i, cell in enumerate(cells):
                for j, w in enumerate(cell):
                    pos[f"{name}[{i}][{j}]"] = w
        wires = [pos[o] for o in order]
        s = 0
        for i, w in enumerate(wires):
            if (before >> i) & 1:
                s |= 1 << w
        final = self._run_perm(s) if self.pure_perm else self._run_exact(s)
        return sum(((final >> w) & 1) << i for i, w in enumerate(wires))


class _Ctx:
    __slots__ = ("fn", "wires", "regions", "values", "pred", "regions_local")


class _Compiler:
    def __init__(self, obj, prog):
        self.obj, self.prog = obj, prog
        self.out = prog.ops
        self.bid = 0

    def ctl_mask(self, ctls):
        m = v = 0
        for w, pol in ctls:
            if (m >> w) & 1 and ((v >> w) & 1) != pol:
                return None
            m |= 1 << w
            if pol:
                v |= 1 << w
        return m, v

    def check(self, wires, value, why):
        m = v = 0
        for i, w in enumerate(wires):
            m |= 1 << w
            if (value >> i) & 1:
                v |= 1 << w
        self.out.append((CHECK, m, v, why))

    def wires_of(self, ctx, tok):
        if '[' in tok:
            nm, idx = tok[:-1].split('[')
            bundle = self.bundle(ctx, nm)
            i = int(idx)
            if i >= len(bundle):
                raise ObjectError("3.3", f"{tok}: index outside the bundle")
            return [bundle[i]]
        return self.bundle(ctx, tok)

    def bundle(self, ctx, nm):
        try:
            return ctx.wires[nm]
        except KeyError:
            raise ObjectError("3.3", f"@{ctx.fn.name}: operand {nm!r} is not a live bundle") from None

    def one(self, ctx, tok):
        w = self.wires_of(ctx, tok)
        if len(w) != 1:
            raise ObjectError("4.4", f"@{ctx.fn.name}: {tok!r} is not a single wire")
        return w[0]

    def gate(self, ctx, ins):
        op, ops = ins.op, ins.operands
        c = list(ctx.pred)
        if op == 'x':
            self.flip(c, self.one(ctx, ops[0]))
        elif op == 'cx':
            self.flip(c + [(self.one(ctx, ops[0]), 1)], self.one(ctx, ops[1]))
        elif op == 'ccx':
            self.flip(c + [(self.one(ctx, ops[0]), 1), (self.one(ctx, ops[1]), 1)], self.one(ctx, ops[2]))
        elif op == 'mcx':
            cs = [(w, 1) for t in parse_list(ops[0]) for w in self.wires_of(ctx, t)]
            self.flip(c + cs, self.one(ctx, ops[1]))
        elif op == 'swap':
            self.exch(c, self.one(ctx, ops[0]), self.one(ctx, ops[1]))
        elif op in PHASE_K:
            t = self.one(ctx, ops[0])
            self.phase(c + [(t, 1)], PHASE_K[op])
        elif op == 'cz':
            self.phase(c + [(self.one(ctx, ops[0]), 1), (self.one(ctx, ops[1]), 1)], 4)
        elif op == 'h':
            mv = self.ctl_mask(c)
            if mv is not None:
                self.prog.pure_perm = False
                self.out.append((H, mv[0], mv[1], self.one(ctx, ops[0])))
        else:
            raise Unchecked(f"gate {op} has no exact Q(zeta_8) basis semantics in this replay "
                            "(a numerical claim covers it)", "4.4")

    def flip(self, ctls, t):
        mv = self.ctl_mask(ctls)
        if mv is not None:
            self.out.append((X, mv[0], mv[1], 1 << t))

    def exch(self, ctls, a, b):
        mv = self.ctl_mask(ctls)
        if mv is not None:
            self.out.append((SWAP, mv[0], mv[1], a, b))

    def phase(self, ctls, k):
        mv = self.ctl_mask(ctls)
        if mv is not None:
            self.prog.pure_perm = False
            self.out.append((PHASE, mv[0], mv[1], k))

    def alloc(self, n):
        w = self.prog.fresh(n)
        self.check(w, 0, "fresh allocation")
        return w

    def free(self, wires, nm):
        self.check(wires, 0, f"qfree {nm}")

    def block(self, ctx, instrs):
        for ins in instrs:
            self.instr(ctx, ins)

    def instr(self, ctx, ins):
        op, ops, lhs = ins.op, ins.operands, ins.lhs
        c = ctx.pred
        if op in PERM_GATES or op in PHASE_K or op in ('h', 'cz', 'rx', 'ry', 'rz'):
            self.gate(ctx, ins)
        elif op == 'qalloc':
            ctx.wires[lhs] = self.alloc(int(ops[1]))
        elif op == 'qfree':
            self.free(self.bundle(ctx, ops[0]), ops[0])
            del ctx.wires[ops[0]]
        elif op == 'qborrow':
            w = self.prog.fresh(int(ops[1]))
            self.bid += 1
            ctx.wires[lhs] = w
            self.out.append((MARK, self.bid, tuple(w)))
        elif op == 'qreturn':
            w = self.bundle(ctx, ops[0])
            self.out.append((MARK, None, tuple(w)))
            del ctx.wires[ops[0]]
        elif op == 'qcopy':
            s = self.bundle(ctx, ops[0])
            d = self.alloc(len(s))
            ctx.wires[lhs] = d
            for a, b in zip(s, d):
                self.flip(c + [(a, 1)], b)
        elif op == 'quncopy':
            s, d = self.bundle(ctx, ops[0]), self.bundle(ctx, ops[1])
            self._same_width(s, d, ins)
            for a, b in zip(s, d):
                self.flip(c + [(a, 1)], b)
            self.free(d, ops[1])
            del ctx.wires[ops[1]]
        elif op == 'qxor':
            s, t = self.bundle(ctx, ops[0]), self.bundle(ctx, ops[1])
            self._same_width(s, t, ins)
            for a, b in zip(s, t):
                self.flip(c + [(a, 1)], b)
        elif op == 'qexch':
            s, t = self.bundle(ctx, ops[0]), self.bundle(ctx, ops[1])
            self._same_width(s, t, ins)
            for a, b in zip(s, t):
                self.exch(c, a, b)
        elif op in ('qload', 'qunload', 'qstore', 'qexchm'):
            self.mem_op(ctx, ins)
        elif op == 'qpred':
            ctx.pred = ctx.pred + [(w, 1) for w in self.wires_of(ctx, ops[0])]
        elif op == 'qunpred':
            n = len(self.wires_of(ctx, ops[0]))
            ctx.pred = ctx.pred[:-n]
        elif op == 'qif':
            w = self.one(ctx, ops[0])
            outer = ctx.pred
            ctx.pred = outer + [(w, 1)]
            self.block(ctx, ins.nested[0])
            ctx.pred = outer
            self.flip(outer, w)
            ctx.pred = outer + [(w, 1)]
            if len(ins.nested) > 1:
                self.block(ctx, ins.nested[1])
            ctx.pred = outer
            self.flip(outer, w)
        elif op == 'qloop':
            self.loop(ctx, ins)
        elif op == 'qinplace':
            self.qinplace(ctx, ins)
        elif op == 'oracle_call':
            self.oracle_call(ctx, ops[0][1:], self.token(ctx, ops[1]),
                             [self.wires_of(ctx, t) for t in parse_list(ops[2])],
                             self.bundle(ctx, ops[3]), self.bundle(ctx, ops[4]),
                             parse_list(ops[5]), parse_list(ops[6]))
        elif op == 'ctl':
            w = self.one(ctx, ops[0])
            inner = _Sub(ops[1], ops[2:], lhs, ins.raw)
            saved = ctx.pred
            ctx.pred = saved + [(w, 1)]
            self.instr(ctx, inner)
            ctx.pred = saved
        elif op == 'adj':
            inner = _Sub(ops[0], ops[1:], lhs, ins.raw)
            self.adjoint(ctx, inner)
        elif op == 'action_call':
            self.action_call(ctx, ops)
        elif op == 'qregion':
            pass  # created at function entry
        else:
            raise Unchecked(f"instruction {op!r} is not implemented by this replay", "5.3")

    def _same_width(self, a, b, ins):
        if len(a) != len(b):
            raise ObjectError("4.5", f"width mismatch in {ins.raw!r}")

    def token(self, ctx, nm):
        try:
            v = ctx.values[nm]
        except KeyError:
            raise ObjectError("4.2", f"{nm!r} is not a classical/token argument of @{ctx.fn.name}") from None
        if not isinstance(v, tuple):
            raise ObjectError("4.7", f"{nm!r} is not a token")
        return v

    def adjoint(self, ctx, inner):
        saved = self.out
        self.out = []
        self.instr(ctx, inner)
        sub, self.out = self.out, saved
        for op in reversed(sub):
            if op[0] == PHASE:
                self.out.append((PHASE, op[1], op[2], (8 - op[3]) % 8))
            else:
                self.out.append(op)

    def loop(self, ctx, ins):
        ops = ins.operands
        n = int(ops[0])
        if len(ops) == 1:
            for _ in range(n):
                self.block(ctx, ins.nested[0])
            return
        if len(ops) != 3 or ops[1] != 'while':
            raise ObjectError("4.6", f"qloop form {ins.raw!r}")
        cond = self.bundle(ctx, ops[2])
        if len(cond) != n:
            raise ObjectError("4.6", f"qloop while: cond width {len(cond)} != {n}")
        flags = self.alloc(n)
        outer = ctx.pred
        self.flip(outer + [(cond[0], 1)], flags[0])
        for i in range(1, n + 1):
            ctx.pred = outer + [(flags[i - 1], 1)]
            self.block(ctx, ins.nested[0])
            ctx.pred = outer
            if i < n:
                self.flip(outer + [(flags[i - 1], 1), (cond[i], 1)], flags[i])
        for i in range(n - 1, 0, -1):
            self.flip(outer + [(flags[i - 1], 1), (cond[i], 1)], flags[i])
        self.flip(outer + [(cond[0], 1)], flags[0])
        self.check(flags, 0, "qloop while flags")

    def mem_op(self, ctx, ins):
        op, ops, lhs = ins.op, ins.operands, ins.lhs
        rname = ops[0]
        if rname not in ctx.regions:
            raise ObjectError("4.6", f"@{ctx.fn.name}: {rname!r} is not a region")
        cells = ctx.regions[rname]
        A = _log2(len(cells))
        addr_tok = ops[1]
        if addr_tok == '()':
            if A != 0:
                raise ObjectError("4.6", "() address on a region with A > 0")
            addr = []
        else:
            addr = self.wires_of(ctx, addr_tok)
            if A == 0 or len(addr) != A:
                raise ObjectError("4.6", f"address actual {addr_tok!r} for A={A}")
        W = len(cells[0])
        if op == 'qload':
            d = self.alloc(W)
            ctx.wires[lhs] = d
        else:
            d = self.bundle(ctx, ops[2])
            if len(d) != W:
                raise ObjectError("4.6", f"value width {len(d)} for W={W}")
        for i, cell in enumerate(cells):
            sel = ctx.pred + [(w, (i >> k) & 1) for k, w in enumerate(addr)]
            for bit in range(W):
                if op in ('qload', 'qunload'):
                    self.flip(sel + [(cell[bit], 1)], d[bit])
                elif op == 'qstore':
                    self.flip(sel + [(d[bit], 1)], cell[bit])
                else:
                    self.exch(sel, cell[bit], d[bit])
        if op == 'qunload':
            self.free(d, ops[2])
            del ctx.wires[ops[2]]

    def oracle_call(self, ctx, cname, tk, ins, tgt, fr, mem_names, arg_names):
        cfn = self.obj.functions.get(cname)
        if cfn is None or cfn.kind != 'oracle':
            raise Unchecked(f"oracle_call target @{cname} is not defined in this object (linking not replayed)", "9.3")
        g = cfn.groups
        widths = [type_width(t, self.obj.ptr_width) for _n, t, _s in g[0]]
        if [len(i) for i in ins] != widths or len(tgt) != type_width(g[1][0][1], self.obj.ptr_width) \
                or len(fr) != type_width(g[2][0][1], self.obj.ptr_width):
            raise ObjectError("4.7", f"oracle_call @{cname}: operand widths do not match the signature")
        if len(mem_names) != len(g[3]) or len(arg_names) != len(g[4]):
            raise ObjectError("4.7", f"oracle_call @{cname}: mem/args arity")
        mem = []
        for rn, (_n, t, _s) in zip(mem_names, g[3]):
            if rn not in ctx.regions:
                raise ObjectError("4.7", f"{rn!r} is not a region of @{ctx.fn.name}")
            if (_log2(len(ctx.regions[rn])), len(ctx.regions[rn][0])) != region_shape(t):
                raise ObjectError("4.7", f"region {rn!r} shape does not match the slot of @{cname}")
            mem.append(ctx.regions[rn])
        args = [self.value(ctx, a) for a in arg_names]
        obj, memo = self.obj, {}

        def fn(iv, rv):
            key = (tuple(iv), tuple(tuple(r) for r in rv))
            hit = memo.get(key)
            if hit is None:
                hit = oracle_payload(obj, cname, iv, rv, args, tk)
                memo[key] = hit
            return hit
        mv = self.ctl_mask(ctx.pred)
        if mv is None:
            return
        self.out.append((ORACLE, mv[0], mv[1], {'ins': ins, 'tgt': tgt, 'fr': fr, 'mem': mem, 'fn': fn}))

    def value(self, ctx, nm):
        try:
            return ctx.values[nm]
        except KeyError:
            raise ObjectError("4.7", f"{nm!r} is not a classical argument of @{ctx.fn.name}") from None

    def qinplace(self, ctx, ins):
        ops = ins.operands
        g, ginv, tk = ops[0][1:], ops[1][1:], self.token(ctx, ops[2])
        x = self.bundle(ctx, ops[3])
        rest = ops[4:] if len(ops) > 4 else ['()', '()', '()']
        if len(rest) != 3:
            raise ObjectError("4.5", f"qinplace form {ins.raw!r}")
        extra_in = [self.wires_of(ctx, t) for t in parse_list(rest[0])]
        mem, args = parse_list(rest[1]), parse_list(rest[2])
        gfn = self.obj.functions.get(g)
        if gfn is None or gfn.kind != 'oracle' or self.obj.functions.get(ginv) is None:
            raise Unchecked(f"qinplace callees @{g}/@{ginv} are not both defined in this object", "9.3")
        F = type_width(gfn.groups[2][0][1], self.obj.ptr_width)
        t, f = self.alloc(len(x)), self.alloc(F)
        all_in = [x] + extra_in
        self.oracle_call(ctx, g, tk, all_in, t, f, mem, args)
        for a, b in zip(x, t):
            self.exch(ctx.pred, a, b)
        self.oracle_call(ctx, ginv, tk, all_in, t, f, mem, args)
        self.check(t, 0, "qinplace private t")
        self.check(f, 0, "qinplace private f")

    def action_call(self, ctx, ops):
        cname = ops[0][1:]
        cfn = self.obj.functions.get(cname)
        if cfn is None or cfn.kind not in ('action', 'perm_action'):
            raise Unchecked(f"action_call target @{cname} is not defined in this object (linking not replayed)", "9.3")
        mut, ins = parse_list(ops[1]), parse_list(ops[2])
        mem, args = parse_list(ops[3]), parse_list(ops[4])
        g = cfn.groups
        if len(mem) != len(g[2]) or len(args) != len(g[3]) or len(mut) != len(g[0]) or len(ins) != len(g[1]):
            raise ObjectError("4.7", f"action_call @{cname}: operand lists do not match the header")
        sub = _Ctx()
        sub.fn = cfn
        sub.wires, sub.regions, sub.values = {}, {}, {}
        sub.pred = list(ctx.pred)
        for tok, (nm, typ, _s) in zip(mut + ins, list(g[0]) + list(g[1])):
            w = self.wires_of(ctx, tok)
            if len(w) != type_width(typ, self.obj.ptr_width):
                raise ObjectError("4.7", f"action_call @{cname}: {tok!r} width does not match {nm}")
            sub.wires[nm] = w
        for rn, (nm, typ, _s) in zip(mem, g[2]):
            if rn not in ctx.regions:
                raise ObjectError("4.7", f"{rn!r} is not a region of @{ctx.fn.name}")
            A, W = region_shape(typ)
            if (_log2(len(ctx.regions[rn])), len(ctx.regions[rn][0])) != (A, W):
                raise ObjectError("4.7", f"region {rn!r} shape does not match slot {nm}")
            sub.regions[nm] = ctx.regions[rn]
        for a, (nm, _t, _s) in zip(args, g[3]):
            sub.values[nm] = self.value(ctx, a)
        self.body(sub)

    def body(self, ctx):
        """§4.6 qregion: created at entry, image gates unconditional, restored at exit."""
        local = []
        for ins in _walk(ctx.fn):
            if ins.op == 'qregion':
                local.append(self.make_region(ctx, ins))
        for _b, _p, instrs in ctx.fn.blocks:
            self.block(ctx, instrs)
        for cells, img in reversed(local):
            for cell, v in zip(cells, img):
                self.check(cell, v, "local region image at exit")
                for i, w in enumerate(cell):
                    if (v >> i) & 1:
                        self.flip([], w)
            for cell in cells:
                self.check(cell, 0, "local region release")

    def make_region(self, ctx, ins):
        ops = ins.operands
        name, A, W = ops[0], int(ops[1]), int(ops[2])
        cells = [self.prog.fresh(W) for _ in range(1 << A)]
        bits = 0
        if len(ops) > 3:
            hx = self.obj.data.get(ops[3][1:])
            if hx is None:
                raise ObjectError("4.6", f"qregion image {ops[3]} has no data symbol")
            bits = int.from_bytes(bytes.fromhex(hx), 'little')
        img = [(bits >> (i * W)) & ((1 << W) - 1) for i in range(1 << A)]
        for cell, v in zip(cells, img):
            self.check(cell, 0, "local region creation")
            for i, w in enumerate(cell):
                if (v >> i) & 1:
                    self.flip([], w)
        ctx.regions[name] = cells
        return cells, img


class _Sub:
    __slots__ = ("op", "operands", "lhs", "raw", "nested")

    def __init__(self, op, operands, lhs, raw):
        self.op, self.operands, self.lhs, self.raw = op, operands, lhs, raw
        self.nested = []


def _walk(fn):
    def rec(instrs):
        for ins in instrs:
            yield ins
            for sub in ins.nested:
                yield from rec(sub)
    for _b, _p, instrs in fn.blocks:
        yield from rec(instrs)


def _log2(n):
    return n.bit_length() - 1


def compile_action(obj, name, args, inherited):
    """Compile one invocation. `args` are resolved values in signature order (tokens as tuples)."""
    fn = obj.functions.get(name)
    if fn is None or fn.kind not in ('action', 'perm_action'):
        raise ObjectError("5.3", f"@{name} is not an action of this object")
    g = fn.groups
    if len(args) != len(g[3]):
        raise ObjectError("12.3", f"@{name}: {len(args)} arguments for {len(g[3])} parameters")
    prog = Program(name)
    prog.nmut = len(g[0])
    prog.controls = [prog.fresh(1) for _ in range(inherited)]
    ctx = _Ctx()
    ctx.fn = fn
    ctx.wires, ctx.regions, ctx.values = {}, {}, {}
    ctx.pred = [(w[0], 1) for w in prog.controls]
    for nm, typ, _s in list(g[0]) + list(g[1]):
        w = prog.fresh(type_width(typ, obj.ptr_width))
        prog.bundles.append(w)
        ctx.wires[nm] = w
        prog.names[nm] = w
    for nm, typ, role in g[2]:
        A, W = region_shape(typ)
        if A > 20:
            raise Unchecked(f"region {nm} has 2^{A} cells; materialising them exceeds this host", "12.2")
        cells = [prog.fresh(W) for _ in range(1 << A)]
        prog.regions.append(cells)
        prog.region_roles.append(role)
        ctx.regions[nm] = cells
        prog.region_names[nm] = cells
    for (nm, typ, _s), v in zip(g[3], args):
        ctx.values[nm] = tuple(int(x) for x in v) if typ == 'token' else v
    comp = _Compiler(obj, prog)
    comp.body(ctx)
    # Resolve borrow lifetimes in final execution order; each bundle's first mark opens it.
    seen, final = set(), []
    for op in prog.ops:
        if op[0] == MARK:
            if op[2] not in seen:
                seen.add(op[2])
                prog.borrow_wires.append(list(op[2]))
                prog.borrow_widths.append(len(op[2]))
        else:
            final.append(op)
    prog.ops = final
    # A live bundle at exit other than the interface is a leak: §5.1.
    leaked = [k for k in ctx.wires if k not in prog.names]
    if leaked:
        raise ObjectError("5.1", f"@{name}: bundles {leaked} are live at exit")
    return prog
