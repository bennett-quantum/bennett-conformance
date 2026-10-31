#!/usr/bin/env python3
"""Neutral-format conformance runner (stdlib only; run as `python3 -I suite/run.py`).

Judges any implementation of the ISA command-line contract against the cases
in suite/cases/<name>/{object.bisa,expect.json}. The format is specified in
suite/FORMAT.md. A case with zero expectations, a malformed expect.json or a
missing file is a failure, never a skip.
"""
import argparse
import json
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
STRIP_ENV = ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "BROWSER")
CLAUSE_RE = re.compile(r"^[A-Z][A-Z0-9]*-\S+$")
RUN_KEYS = {"cmd", "stdout_json", "stdout", "exit", "stderr_contains"}
TOP_KEYS = {"spec", "source", "clauses", "hashes", "check", "runs"}
TIMEOUT_S = 300


class CaseError(Exception):
    """The case itself is malformed (distinct from an implementation mismatch)."""


def reject_constant(name):
    raise ValueError("non-JSON constant %s" % name)


def parse_json(text):
    return json.loads(text, parse_constant=reject_constant)


def jeq(a, b):
    """Structural JSON equality; unlike Python ==, true is not 1."""
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool) and a == b
    if isinstance(a, dict):
        return (isinstance(b, dict) and a.keys() == b.keys()
                and all(jeq(a[k], b[k]) for k in a))
    if isinstance(a, list):
        return (isinstance(b, list) and len(a) == len(b)
                and all(jeq(x, y) for x, y in zip(a, b)))
    if isinstance(a, (int, float)):
        return isinstance(b, (int, float)) and a == b
    return type(a) is type(b) and a == b


def load_case(case_dir):
    """Return the validated expect.json dict or raise CaseError."""
    obj = os.path.join(case_dir, "object.bisa")
    exp = os.path.join(case_dir, "expect.json")
    if not os.path.isfile(obj):
        raise CaseError("missing object.bisa")
    if not os.path.isfile(exp):
        raise CaseError("missing expect.json")
    try:
        with open(exp, encoding="utf-8") as f:
            e = parse_json(f.read())
    except ValueError as err:
        raise CaseError("expect.json is not valid JSON: %s" % err)
    if not isinstance(e, dict):
        raise CaseError("expect.json must be an object")
    extra = set(e) - TOP_KEYS
    if extra:
        raise CaseError("unknown expect.json keys: %s" % sorted(extra))
    for k in ("spec", "source"):
        if not isinstance(e.get(k), str) or not e[k]:
            raise CaseError("expect.json needs a non-empty string %r" % k)
    cl = e.get("clauses")
    if (not isinstance(cl, list) or not cl
            or not all(isinstance(c, str) and CLAUSE_RE.match(c) for c in cl)):
        raise CaseError("clauses must be a non-empty list of tags like ISA-4.5")
    h = e.get("hashes")
    if h is not None:
        if (not isinstance(h, dict) or set(h) != {"object", "symbols"}
                or not isinstance(h["object"], str)
                or not isinstance(h["symbols"], dict)):
            raise CaseError("hashes must be {object, symbols}")
        for n, v in h["symbols"].items():
            if not isinstance(v, dict) or set(v) != {"interface", "closure"}:
                raise CaseError("hashes.symbols.%s must be {interface, closure}" % n)
    c = e.get("check")
    if c is not None and c != "ok":
        if (not isinstance(c, dict) or set(c) - {"exit", "stderr_contains"}
                or not isinstance(c.get("exit"), int) or c["exit"] == 0
                or not isinstance(c.get("stderr_contains"), str)):
            raise CaseError('check must be "ok" or {exit (nonzero), stderr_contains}')
    runs = e.get("runs")
    if runs is not None:
        if not isinstance(runs, list):
            raise CaseError("runs must be a list")
        for i, r in enumerate(runs):
            if not isinstance(r, dict) or set(r) - RUN_KEYS:
                raise CaseError("runs[%d]: unknown or non-object entry" % i)
            if (not isinstance(r.get("cmd"), list) or not r["cmd"]
                    or not all(isinstance(a, str) for a in r["cmd"])):
                raise CaseError("runs[%d].cmd must be a non-empty string list" % i)
            if not isinstance(r.get("exit"), int):
                raise CaseError("runs[%d].exit must be an integer" % i)
            if ("stdout_json" in r) == ("stdout" in r):
                raise CaseError("runs[%d] needs exactly one of stdout_json, stdout" % i)
            if "stdout" in r and not isinstance(r["stdout"], str):
                raise CaseError("runs[%d].stdout must be a string" % i)
            sc = r.get("stderr_contains")
            if sc is not None and not (isinstance(sc, str) or (
                    isinstance(sc, list) and all(isinstance(s, str) for s in sc))):
                raise CaseError("runs[%d].stderr_contains must be string or list" % i)
            if r["exit"] != 0 and sc is None:
                raise CaseError("runs[%d]: a nonzero exit needs stderr_contains" % i)
    if h is None and c is None and not runs:
        raise CaseError("case has zero expectations (no hashes, check or runs)")
    return e


def invoke(binary, args, case_dir):
    env = {k: v for k, v in os.environ.items() if k not in STRIP_ENV}
    try:
        p = subprocess.run([binary] + args, cwd=case_dir, env=env,
                           capture_output=True, timeout=TIMEOUT_S)
    except subprocess.TimeoutExpired:
        return None, "", "timed out after %ds" % TIMEOUT_S
    except OSError as err:
        raise CaseError("cannot execute binary: %s" % err)
    return (p.returncode, p.stdout.decode("utf-8", "replace"),
            p.stderr.decode("utf-8", "replace"))


def subst(arg, case_dir):
    obj = os.path.join(case_dir, "object.bisa")
    return arg.replace("$OBJECT", obj).replace("$CASE", case_dir)


def check_hashes(binary, case_dir, expected):
    code, out, err = invoke(binary, ["hash", os.path.join(case_dir, "object.bisa")], case_dir)
    if code != 0:
        return ["hash exited %r: %s" % (code, err.strip()[:200])]
    return compare_hash_output(out, expected)


def compare_hash_output(out, expected):
    try:
        got = parse_json(out)
    except ValueError as err:
        return ["hash output is not JSON: %s" % err]
    problems = []
    if not isinstance(got, dict) or not isinstance(got.get("symbols"), dict):
        return ["hash output lacks object_hash/symbols"]
    if got.get("object_hash") != expected["object"]:
        problems.append("object hash %r != expected %r" % (got.get("object_hash"), expected["object"]))
    want, have = expected["symbols"], got["symbols"]
    for n in sorted(set(want) | set(have)):
        if n not in have:
            problems.append("symbol %s missing from output" % n)
        elif n not in want:
            problems.append("symbol %s not in expectations" % n)
        else:
            for k, f in (("interface", "interface_hash"), ("closure", "closure_hash")):
                if have[n].get(f) != want[n][k]:
                    problems.append("%s %s %r != expected %r" % (n, k, have[n].get(f), want[n][k]))
    return problems


def check_check(binary, case_dir, expected, hashes):
    code, out, err = invoke(binary, ["check", os.path.join(case_dir, "object.bisa")], case_dir)
    if expected == "ok":
        if code != 0:
            return ["check exited %r: %s" % (code, err.strip()[:200])]
        try:
            got = parse_json(out)
        except ValueError as e:
            return ["check output is not JSON: %s" % e]
        problems = []
        if not isinstance(got, dict) or got.get("status") != "ok":
            problems.append("check status %r != 'ok'" % (got.get("status") if isinstance(got, dict) else got))
        if hashes is not None:
            problems += compare_hash_output(out, hashes)
        return problems
    problems = []
    if code != expected["exit"]:
        problems.append("check exit %r != expected %r" % (code, expected["exit"]))
    if expected["stderr_contains"] not in err:
        problems.append("check stderr lacks %r" % expected["stderr_contains"])
    return problems


def check_run(binary, case_dir, run):
    args = [subst(a, case_dir) for a in run["cmd"]]
    code, out, err = invoke(binary, args, case_dir)
    problems = []
    if code != run["exit"]:
        problems.append("exit %r != expected %r (stderr: %s)" % (code, run["exit"], err.strip()[:200]))
    if "stdout" in run:
        if out != run["stdout"]:
            problems.append("stdout differs: got %r, expected %r" % (out[:200], run["stdout"][:200]))
    else:
        try:
            got = parse_json(out)
        except ValueError as e:
            problems.append("stdout is not JSON: %s (%r)" % (e, out[:120]))
        else:
            if not jeq(got, run["stdout_json"]):
                problems.append("stdout_json differs:\n    got      %s\n    expected %s" % (
                    json.dumps(got, sort_keys=True)[:600],
                    json.dumps(run["stdout_json"], sort_keys=True)[:600]))
    needles = run.get("stderr_contains", [])
    for n in [needles] if isinstance(needles, str) else needles:
        if n not in err:
            problems.append("stderr lacks %r (stderr: %s)" % (n, err.strip()[:200]))
    return problems


def run_case(binary, case_dir):
    """Return (cells, problems); cells are the table columns hashes/check/runs."""
    e = load_case(case_dir)
    cells = {"hashes": "-", "check": "-", "runs": "-"}
    problems = []
    if e.get("hashes") is not None:
        p = check_hashes(binary, case_dir, e["hashes"])
        cells["hashes"] = "FAIL" if p else "ok"
        problems += ["hashes: " + x for x in p]
    if e.get("check") is not None:
        p = check_check(binary, case_dir, e["check"], e.get("hashes"))
        cells["check"] = "FAIL" if p else "ok"
        problems += ["check: " + x for x in p]
    runs = e.get("runs") or []
    if runs:
        passed = 0
        for i, r in enumerate(runs):
            p = check_run(binary, case_dir, r)
            passed += not p
            problems += ["runs[%d] %s: %s" % (i, " ".join(r["cmd"][:1] + r["cmd"][2:3]), x) for x in p]
        cells["runs"] = "%d/%d" % (passed, len(runs))
    return cells, problems


def list_cases(cases_dir):
    if not os.path.isdir(cases_dir):
        raise SystemExit("run.py: no such cases directory: %s" % cases_dir)
    names = sorted(n for n in os.listdir(cases_dir)
                   if os.path.isdir(os.path.join(cases_dir, n)))
    if not names:
        raise SystemExit("run.py: no cases found in %s" % cases_dir)
    return names


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--binary", default=os.environ.get("BENNETT_ISA_BINARY"),
                    help="implementation under test (default: $BENNETT_ISA_BINARY)")
    ap.add_argument("--cases", default=os.path.join(HERE, "cases"))
    ap.add_argument("--tags-json", action="store_true",
                    help="print {case: [clauses]} and exit; runs nothing")
    a = ap.parse_args(argv)
    names = list_cases(a.cases)

    if a.tags_json:
        tags = {}
        for n in names:
            tags[n] = load_case_for_tags(os.path.join(a.cases, n), n)
        json.dump(tags, sys.stdout, indent=1, sort_keys=True)
        sys.stdout.write("\n")
        return 0

    if not a.binary:
        raise SystemExit("run.py: --binary (or $BENNETT_ISA_BINARY) is required")
    if not os.access(a.binary, os.X_OK):
        raise SystemExit("run.py: binary not executable: %s" % a.binary)
    binary = os.path.abspath(a.binary)

    rows, details, good = [], [], 0
    for n in names:
        d = os.path.abspath(os.path.join(a.cases, n))
        try:
            cells, problems = run_case(binary, d)
        except CaseError as err:
            cells, problems = {"hashes": "-", "check": "-", "runs": "-"}, ["case: %s" % err]
        ok = not problems
        good += ok
        rows.append((n, cells["hashes"], cells["check"], cells["runs"], "PASS" if ok else "FAIL"))
        details += ["%s: %s" % (n, p) for p in problems]
    head = ("case", "hashes", "check", "runs", "result")
    w = [max(len(r[i]) for r in rows + [head]) for i in range(5)]
    for r in [head] + rows:
        print(" | ".join(c.ljust(w[i]) for i, c in enumerate(r)).rstrip())
    for line in details:
        print(line)
    print("checked %d/%d cases" % (good, len(names)))
    return 0 if good == len(names) else 1


def load_case_for_tags(case_dir, name):
    try:
        return load_case(case_dir)["clauses"]
    except CaseError as err:
        raise SystemExit("run.py: case %s: %s" % (name, err))


if __name__ == "__main__":
    sys.exit(main())
