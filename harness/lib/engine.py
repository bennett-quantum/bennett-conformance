"""Runs cases through N executors, compares every pair and keeps exact, total accounting.

Classes per case per pair: MATCH_EXACT, MATCH_TOLERANCE, AGREE_REJECT, UNSUPPORTED, TIMEOUT, DIVERGE.
Every case lands in exactly one class for each pair; whatever is not run is a counted skip with a reason.
Exit status: 0 clean; 1 divergence, oracle failure or an empty run; 2 nothing failed but something was
unsupported, timed out or skipped.
"""
import concurrent.futures
import hashlib
import itertools
import json
import os
import shutil
import threading

import compare
import shrink

CLASSES = ("MATCH_EXACT", "MATCH_TOLERANCE", "AGREE_REJECT", "UNSUPPORTED", "TIMEOUT", "DIVERGE")


def subset_expect(code, value):
    return {"exit": code, "stdout_json": value}


class Case:
    def __init__(self, corpus, case_id, argv, object_text=None, object_path=None, cwd=None, expect=None,
                 alt_expect=None, tape=False, unavailable=False):
        self.corpus, self.id, self.argv = corpus, case_id, list(argv)
        self.object_text, self.object_path, self.cwd = object_text, object_path, cwd
        self.expect, self.alt_expect, self.tape, self.unavailable = expect, alt_expect, tape, unavailable


class PairResult:
    def __init__(self, cls, diff=None, signature=None):
        self.cls, self.diff, self.signature, self.witness = cls, diff or compare.Diff(), signature, None


class CaseResult:
    def __init__(self, case, outcomes):
        self.case, self.outcomes, self.pairs = case, outcomes, {}


def _delta(path, a, b, mode="exact"):
    return compare.Delta(path, a, b, mode)


def compare_pair(a, b, cmd):
    """Class and differences of one pair of outcomes (with tape and cross-replay observations)."""
    d = compare.Diff()
    bad = {"crash", "badoutput"}
    if a.kind in bad or b.kind in bad:
        d.hard.append(_delta("$outcome", _brief(a), _brief(b)))
    elif "timeout" in (a.kind, b.kind):
        return PairResult("TIMEOUT", d)
    elif "unsupported" in (a.kind, b.kind):
        return PairResult("UNSUPPORTED", d)
    elif a.kind == "diagnostic" and b.kind == "diagnostic":
        return PairResult("AGREE_REJECT", d)
    elif a.kind != b.kind:
        d.hard.append(_delta("$outcome", _brief(a), _brief(b)))
    else:
        inner = compare.compare_json(compare.parse_json(a.stdout), compare.parse_json(b.stdout))
        d.hard.extend(inner.hard)
        d.tolerated.extend(inner.tolerated)
        d.relational.extend(inner.relational)
        if a.tape is not None and b.tape is not None and a.tape != b.tape:
            d.hard.append(_delta("$tape_bytes", a.tape.decode("utf-8", "replace"), b.tape.decode("utf-8", "replace")))
        for side, o in (("a", a), ("b", b)):
            if o.replay_ok is False:
                d.hard.append(_delta("$cross_replay", side, o.replay_note))
    if d.hard:
        return PairResult("DIVERGE", d, signature(cmd, a, b, d))
    return PairResult("MATCH_TOLERANCE" if d.tolerated else "MATCH_EXACT", d)


def _brief(o):
    return {"kind": o.kind, "exit": o.code, "stderr": o.stderr[:300], "stdout": o.stdout[:300].decode("utf-8", "replace")}


def signature(cmd, a, b, d):
    fields = sorted({compare.collapse(x.path) for x in d.hard})
    rules = ""
    if "$outcome" in fields:
        rules = "%s|%s" % (compare.rule_token(a.stderr), compare.rule_token(b.stderr))
    return "%s %s/%s %s %s" % (cmd, a.kind, b.kind, ",".join(fields), rules)


def replay_argv(argv, tape_path):
    return list(argv) + ["--replay-tape", tape_path]


class Runner:
    def __init__(self, execs, workdir, witness_dir=None, shrink_seconds=20.0, witnesses_per_signature=2,
                 pairs=None, audit=None):
        if len(execs) < 2:
            raise ValueError("a differential run needs at least two executors")
        names = [e.name for e in execs]
        if len(set(names)) != len(names):
            raise ValueError("duplicate executor names %s" % names)
        self.execs = execs
        self.pairs = pairs or list(itertools.combinations(names, 2))
        self.by_name = {e.name: e for e in execs}
        self.workdir, self.witness_dir = workdir, witness_dir
        self.shrink_seconds, self.witnesses_per_signature = shrink_seconds, witnesses_per_signature
        os.makedirs(workdir, exist_ok=True)
        self.lock = threading.Lock()
        self.counter = itertools.count()
        self.corpora = {}
        self.divergences = {}
        self.oracle_failures = []
        self.unsupported = {}
        self.timeouts = []
        self.reject_rule_differs = 0
        self.byte_identical = 0
        self.json_pairs = 0
        self.audit = audit or {}
        self.audit_hits = {}
        self.generator_rejections = []
        self.witnesses_made = {}

    # --- accounting ---
    def _corpus(self, name):
        return self.corpora.setdefault(name, {
            "cases": 0, "skipped": {}, "pairs": {"%s:%s" % p: {} for p in self.pairs},
            "oracle": {e.name: {} for e in self.execs}, "kinds": {e.name: {} for e in self.execs}})

    def skip(self, corpus, case_id, reason):
        c = self._corpus(corpus)
        c["skipped"][reason] = c["skipped"].get(reason, 0) + 1

    # --- execution ---
    def _object_path(self, text):
        name = hashlib.sha256(text.encode("utf-8")).hexdigest() + ".bisa"
        d = os.path.join(self.workdir, "objects")
        os.makedirs(d, exist_ok=True)
        path = os.path.join(d, name)
        if not os.path.exists(path):
            tmp = "%s.%d.tmp" % (path, next(self.counter))
            with open(tmp, "w", encoding="utf-8", newline="") as f:
                f.write(text)
            os.replace(tmp, path)
        return path

    def _resolve(self, case):
        path = case.object_path
        if case.object_text is not None:
            path = self._object_path(case.object_text)
        return [path if a == "$OBJECT" else a for a in case.argv]

    def execute(self, case, names=None):
        argv = self._resolve(case)
        execs = [self.by_name[n] for n in names] if names else self.execs
        scratch = None
        outcomes = {}
        try:
            if case.tape:
                scratch = os.path.join(self.workdir, "tapes", "%d" % next(self.counter))
                os.makedirs(scratch)
            for e in execs:
                args = list(argv)
                tape = None
                if case.tape:
                    tape = os.path.join(scratch, e.name + ".tape")
                    args += ["--tape-out", tape]
                o = e.run(args, case.cwd)
                if tape and o.kind == "json":
                    if not os.path.exists(tape):
                        o.kind = "badoutput"
                        o.stderr += "\nharness: exit 0 but no --tape-out file written"
                    else:
                        with open(tape, "rb") as f:
                            o.tape = f.read()
                outcomes[e.name] = o
            if case.tape:
                self._cross_replay(execs, argv, outcomes, scratch, case.cwd)
        finally:
            if scratch:
                shutil.rmtree(scratch, ignore_errors=True)
        return outcomes

    def _cross_replay(self, execs, argv, outcomes, scratch, cwd):
        """Each executor replays its neighbour's saved tape: stdout must stay byte-identical (profile v1)."""
        for i, e in enumerate(execs):
            peer = execs[(i + 1) % len(execs)]
            mine, theirs = outcomes[e.name], outcomes[peer.name]
            if mine.kind != "json" or theirs.kind != "json" or not theirs.tape or not theirs.tape.strip(b"[] \n"):
                continue
            o = e.run(replay_argv(argv, os.path.join(scratch, peer.name + ".tape")), cwd)
            mine.replay_ok = o.kind == "json" and o.stdout == mine.stdout
            if not mine.replay_ok:
                mine.replay_note = "%s replaying %s's tape: %s %s" % (e.name, peer.name, o.kind, o.stderr[:200])

    def result_of(self, case, outcomes):
        res = CaseResult(case, outcomes)
        cmd = case.argv[0] if case.argv else "?"
        for a, b in self.pairs:
            if a in outcomes and b in outcomes:
                res.pairs[(a, b)] = compare_pair(outcomes[a], outcomes[b], cmd)
        return res

    def account(self, res):
        case = res.case
        c = self._corpus(case.corpus)
        c["cases"] += 1
        for name, o in res.outcomes.items():
            c["kinds"][name][o.kind] = c["kinds"][name].get(o.kind, 0) + 1
            if o.kind == "unsupported":
                reason = next((x for x in o.stderr.strip().splitlines() if compare.UNSUPPORTED.search(x)), o.stderr)[:160]
                table = self.unsupported.setdefault(name, {})
                table[reason] = table.get(reason, 0) + 1
            elif o.kind == "timeout":
                self.timeouts.append({"case": case.id, "corpus": case.corpus, "executor": name})
            pattern = self.audit.get(name)
            if pattern and pattern.search(o.stderr):
                self.audit_hits[name] = self.audit_hits.get(name, 0) + 1
            if case.expect is not None:
                self._judge(c, case, name, o)
        for pair, pr in res.pairs.items():
            row = c["pairs"]["%s:%s" % pair]
            row[pr.cls] = row.get(pr.cls, 0) + 1
            a, b = res.outcomes[pair[0]], res.outcomes[pair[1]]
            if pr.cls == "AGREE_REJECT" and compare.rule_token(a.stderr) != compare.rule_token(b.stderr):
                self.reject_rule_differs += 1
            if a.kind == "json" and b.kind == "json":
                self.json_pairs += 1
                if a.stdout == b.stdout:
                    self.byte_identical += 1
            if pr.cls == "DIVERGE":
                self._divergence(case, pair, pr, a, b)

    def _judge(self, c, case, name, o):
        row = c["oracle"][name]
        problems = compare.judge(case.expect, o.code, o.stdout, o.stderr)
        verdict = "pass"
        if case.unavailable:
            alt = compare.judge(case.alt_expect, o.code, o.stdout, o.stderr) if case.alt_expect else ["no alternative"]
            verdict = "unavailable" if (not problems or not alt) else "fail"
        elif problems:
            verdict = "fail"
        row[verdict] = row.get(verdict, 0) + 1
        if verdict == "fail":
            self.oracle_failures.append({"corpus": case.corpus, "case": case.id, "executor": name,
                                         "argv": case.argv, "problem": "; ".join(problems)[:400],
                                         "stderr": o.stderr[:300]})

    def _divergence(self, case, pair, pr, a, b):
        key = "%s:%s %s" % (pair[0], pair[1], pr.signature)
        row = self.divergences.setdefault(key, {"pair": "%s:%s" % pair, "signature": pr.signature, "count": 0,
                                                "first_case": "%s/%s" % (case.corpus, case.id), "witnesses": []})
        row["count"] += 1
        # One quota per signature across pairs: engines of one executor usually share a root cause.
        made = self.witnesses_made.get(pr.signature, [])
        if self.witness_dir and len(made) < self.witnesses_per_signature:
            pr.witness = shrink.write_witness(self, case, pair, pr, self.witness_dir, self.shrink_seconds)
            made.append(pr.witness)
            self.witnesses_made[pr.signature] = made
        row["witnesses"] = list(made)

    def run_case(self, case):
        res = self.result_of(case, self.execute(case))
        self.account(res)
        return res

    def run_all(self, cases, jobs=4):
        """Execute in parallel; account and shrink strictly in input order, so results are deterministic."""
        cases = list(cases)
        with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as pool:
            for case, outcomes in zip(cases, pool.map(self.execute, cases)):
                self.account(self.result_of(case, outcomes))

    def note_generator_rejection(self, cls, case, outcomes):
        """A generated program every executor refuses is a generator defect or a shared gap: reported, kept."""
        first = next(iter(outcomes.values()))
        self.generator_rejections.append({"class": cls, "case": case.id, "stderr": first.stderr[:300]})

    # --- reporting ---
    def summary(self):
        totals = {"cases": 0, "skipped": 0, "diverge": 0, "oracle_fail": 0, "unsupported": 0, "timeout": 0,
                  "agree_reject": 0, "match_exact": 0, "match_tolerance": 0}
        for c in self.corpora.values():
            totals["cases"] += c["cases"]
            totals["skipped"] += sum(c["skipped"].values())
            for row in c["pairs"].values():
                expected = c["cases"]
                if sum(row.values()) != expected:
                    raise AssertionError("accounting: pair row %r does not total %d cases" % (row, expected))
                totals["diverge"] += row.get("DIVERGE", 0)
                totals["unsupported"] += row.get("UNSUPPORTED", 0)
                totals["timeout"] += row.get("TIMEOUT", 0)
                totals["agree_reject"] += row.get("AGREE_REJECT", 0)
                totals["match_exact"] += row.get("MATCH_EXACT", 0)
                totals["match_tolerance"] += row.get("MATCH_TOLERANCE", 0)
            for row in c["oracle"].values():
                totals["oracle_fail"] += row.get("fail", 0)
        return {
            "executors": {e.name: e.describe() for e in self.execs},
            "pairs": ["%s:%s" % p for p in self.pairs],
            "corpora": self.corpora,
            "totals": totals,
            "divergences": sorted(self.divergences.values(), key=lambda r: (-r["count"], r["pair"], r["signature"])),
            "oracle_failures": self.oracle_failures,
            "unsupported_reasons": self.unsupported,
            "timeouts": self.timeouts,
            "agree_reject_rule_token_differs": self.reject_rule_differs,
            "json_pairs_compared": self.json_pairs,
            "json_pairs_byte_identical": self.byte_identical,
            "audit_hits": self.audit_hits,
            "generator_rejections": self.generator_rejections,
        }

    def exit_status(self):
        t = self.summary()["totals"]
        if t["diverge"] or t["oracle_fail"] or (t["cases"] == 0):
            return 1
        if t["unsupported"] or t["timeout"] or t["skipped"]:
            return 2
        return 0


def dump(summary, path):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=1, sort_keys=True)
        f.write("\n")
