"""Witness minimisation: greedy, deterministic reduction of an (object, argv) pair under a signature.

Reductions: drop option groups; zero option values; drop symbols; ddmin function lines; drop regs entries;
reduce literals. Every candidate object is rehashed per §9.2. The predicate requires the same divergence
signature, so a reduction that makes both executors reject (or moves the difference) is not accepted.
"""
import hashlib
import json
import os
import re
import time

import compare
import identity
import objfile

CONST = re.compile(r"^(\S+ = (?:lift\.)?const\.i\d+) (\d+)$")
REGS = re.compile(r"^regs \{ (.*) \}(.*)$")


def object_shrink_blocker(text):
    """Why the object cannot be reduced with recomputed identity, or None when it can."""
    try:
        o = objfile.parse(text)
        bad = identity.stored_mismatches(o)
    except (objfile.ObjectError, identity.IdentityError, KeyError, ValueError) as e:
        return "object does not parse for identity: %s" % e
    if bad:
        return "stored hashes do not match §9.2 recomputation: %s" % bad[0]
    return None


class _Budget(Exception):
    pass


class Shrinker:
    def __init__(self, runner, case, pair, seconds, target):
        self.runner, self.case, self.pair, self.target = runner, case, pair, target
        self.deadline = time.monotonic() + seconds
        self.tests = 0
        self.seen = {}

    def holds(self, text, argv):
        key = (text, tuple(argv))
        if key in self.seen:
            return self.seen[key]
        if time.monotonic() > self.deadline:
            raise _Budget()
        import engine
        self.tests += 1
        c = engine.Case(self.case.corpus, self.case.id, argv, object_text=text, object_path=self.case.object_path,
                        cwd=self.case.cwd, tape=self.case.tape)
        outcomes = self.runner.execute(c, list(self.pair))
        pr = engine.compare_pair(outcomes[self.pair[0]], outcomes[self.pair[1]], argv[0])
        ok = pr.cls == "DIVERGE" and pr.signature == self.target
        self.seen[key] = ok
        return ok

    # --- argv ---
    def shrink_argv(self, text, argv):
        first = next((i for i, a in enumerate(argv) if a.startswith("--")), len(argv))
        changed = True
        while changed:
            changed = False
            groups = self._groups(argv, first)
            for start, end in reversed(groups):
                cand = argv[:start] + argv[end:]
                if self.holds(text, cand):
                    argv = cand
                    changed = True
                    break
            if changed:
                continue
            for i in range(first, len(argv)):
                if argv[i].startswith("--") or argv[i] in ("$OBJECT",):
                    continue
                for value in self._simpler_values(argv[i]):
                    cand = argv[:i] + [value] + argv[i + 1:]
                    if value != argv[i] and self.holds(text, cand):
                        argv = cand
                        changed = True
                        break
                if changed:
                    break
        return argv

    @staticmethod
    def _groups(argv, first):
        out, i = [], first
        while i < len(argv):
            j = i + 1
            while j < len(argv) and not argv[j].startswith("--"):
                j += 1
            out.append((i, j))
            i = j
        return out

    @staticmethod
    def _simpler_values(v):
        if re.fullmatch(r"\d+", v) and int(v) > 1:
            return ["0", "1", str(int(v) // 2)]
        if re.fullmatch(r"[\d,]+", v) and "," in v:
            return [",".join("0" for _ in v.split(","))]
        return []

    # --- object ---
    def shrink_object(self, text, argv):
        entry = argv[2] if len(argv) > 2 and not argv[2].startswith("--") else None
        entry = entry.lstrip("@") if entry else None
        changed = True
        while changed:
            changed = False
            new = self._drop_symbols(text, argv, entry)
            changed |= new != text
            text = new
            new = self._ddmin_lines(text, argv)
            changed |= new != text
            text = new
            new = self._drop_regs(text, argv)
            changed |= new != text
            text = new
            new = self._reduce_literals(text, argv)
            changed |= new != text
            text = new
        return text

    def _rebuild(self, o):
        try:
            return identity.rehash(o).text()
        except (identity.IdentityError, KeyError, ValueError, objfile.ObjectError):
            return None

    def _try(self, o, argv):
        cand = self._rebuild(o)
        return cand if cand is not None and self.holds(cand, argv) else None

    def _drop_symbols(self, text, argv, entry):
        o = objfile.parse(text)
        for name in sorted(s for s in o.symbol_table() if s != entry):
            c = objfile.parse(text)
            table = c.symbols()
            rest = [v for v in (table[k] for k in sorted(table, key=int)) if v["name"] != name]
            c.set_section("symbols", {str(i): v for i, v in enumerate(rest)})
            c.functions = [f for f in c.functions if f.name != name]
            c.data = [d for d in c.data if d[0] != name]
            lay = c.layout()
            if lay and name in (lay.get("globals") or {}):
                del lay["globals"][name]
                c.set_section("layout", lay)
            got = self._try(c, argv)
            if got is not None:
                text = got
        return text

    def _ddmin_lines(self, text, argv):
        names = [f.name for f in objfile.parse(text).functions]
        for name in names:
            size = max(len(objfile.parse(text).function(name).lines) // 2, 1)
            while True:
                f = objfile.parse(text).function(name)
                i, removed = 1, False
                while i < len(f.lines):
                    c = objfile.parse(text)
                    g = c.function(name)
                    g.lines = g.lines[:i] + g.lines[i + size:]
                    got = self._try(c, argv) if len(g.lines) >= 1 else None
                    if got is not None:
                        text = got
                        f = objfile.parse(text).function(name)
                        removed = True
                    else:
                        i += size
                if size == 1 and not removed:
                    break
                size = max(size // 2, 1)
        return text

    def _drop_regs(self, text, argv):
        for f in objfile.parse(text).functions:
            while True:
                g = objfile.parse(text).function(f.name)
                idx = next((i for i, x in enumerate(g.lines) if REGS.match(x)), None)
                if idx is None:
                    break
                m = REGS.match(g.lines[idx])
                entries = [e for e in m.group(1).split(", ") if e]
                done = True
                for k in range(len(entries)):
                    c = objfile.parse(text)
                    rest = entries[:k] + entries[k + 1:]
                    c.function(f.name).lines[idx] = "regs { %s }%s" % (", ".join(rest), m.group(2))
                    got = self._try(c, argv)
                    if got is not None:
                        text = got
                        done = False
                        break
                if done:
                    break
        return text

    def _reduce_literals(self, text, argv):
        for f in objfile.parse(text).functions:
            n = len(objfile.parse(text).function(f.name).lines)
            for i in range(n):
                g = objfile.parse(text).function(f.name)
                m = CONST.match(g.lines[i])
                if not m or int(m.group(2)) <= 1:
                    continue
                for value in ("0", "1"):
                    c = objfile.parse(text)
                    c.function(f.name).lines[i] = "%s %s" % (m.group(1), value)
                    got = self._try(c, argv)
                    if got is not None:
                        text = got
                        break
        return text


def write_witness(runner, case, pair, pr, witness_dir, seconds):
    text = case.object_text
    if text is None and case.object_path:
        with open(case.object_path, encoding="utf-8", newline="") as f:
            text = f.read()
    argv = list(case.argv)
    s = Shrinker(runner, case, pair, seconds, pr.signature)
    status = "complete"
    original = len(text.splitlines()) if text else 0
    blocker = object_shrink_blocker(text) if text else "case has no object"
    has_object = text is not None and "$OBJECT" in argv
    try:
        if has_object or text is None:
            argv = s.shrink_argv(text, argv)
        if has_object and blocker is None:
            text = s.shrink_object(text, argv)
            argv = s.shrink_argv(text, argv)
    except _Budget:
        status = "time budget of %gs reached; witness valid but maybe not minimal" % seconds
    if blocker:
        status = "object reduction unavailable: %s" % blocker
    final = _final_run(runner, case, pair, text, argv)
    digest = hashlib.sha256(("%s\n%s\n%s" % (pr.signature, " ".join(argv), text)).encode("utf-8")).hexdigest()[:12]
    d = os.path.join(witness_dir, "%s-%s-%s" % (re.sub(r"[^a-z0-9]+", "-", case.corpus.lower()), "-".join(pair), digest))
    os.makedirs(d, exist_ok=True)
    if text is not None:
        with open(os.path.join(d, "object.bisa"), "w", encoding="utf-8", newline="") as f:
            f.write(text)
    a, b = final[pair[0]], final[pair[1]]
    import engine
    fp = engine.compare_pair(a, b, argv[0])
    meta = {
        "pair": list(pair),
        "case": "%s/%s" % (case.corpus, case.id),
        "signature": pr.signature,
        "signature_reproduced": fp.signature == pr.signature,
        "argv": argv,
        "cwd": case.cwd,
        "tape": case.tape,
        "executors": {n: runner.by_name[n].describe() for n in pair},
        "diff": [x.as_json() for x in fp.diff.hard] or [x.as_json() for x in pr.diff.hard],
        "outcomes": {n: _clip_outcome(final[n]) for n in pair},
        "original_lines": original,
        "shrunk_lines": len(text.splitlines()) if text else 0,
        "shrink_tests": s.tests,
        "shrink_status": status,
    }
    with open(os.path.join(d, "witness.json"), "w", encoding="utf-8") as f:
        json.dump(meta, f, indent=1, sort_keys=True)
        f.write("\n")
    return d


def _clip_outcome(o):
    return {"kind": o.kind, "exit": o.code, "stdout": o.stdout[:2000].decode("utf-8", "replace"),
            "stderr": o.stderr[:1000]}


def _final_run(runner, case, pair, text, argv):
    import engine
    c = engine.Case(case.corpus, case.id, argv, object_text=text, object_path=case.object_path, cwd=case.cwd,
                    tape=case.tape)
    return runner.execute(c, list(pair))
