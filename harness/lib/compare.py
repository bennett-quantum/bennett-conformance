"""Outcome comparison. Every field is compared under a labelled mode; unknown fields default to exact.

Modes
  exact       structural JSON equality (true is not 1; numbers by value; binary64 by bits).
  tolerance   amplitudes under `state`/`forward_state`: |a-b| <= AMPLITUDE_TOLERANCE, reported separately
              from exact agreement. A component missing on one side counts as amplitude 0.
  relational  the digest of the §7.1 memory is opaque (no ISA serialisation), so bytes are not compared
              across executors; only whether memory_digest == entry_memory_digest on each side.
  nullness    first_difference is descriptive text; only null versus non-null is contractual.
"""
import json
import re
import struct

AMPLITUDE_TOLERANCE = 1e-12
TOLERANT_ROOTS = {"state", "forward_state"}
UNSUPPORTED = re.compile(r"\bunsupported\b(?! ISA version)|\bunimplemented\b|staged deliverable|not implemented"
                         r"|not yet (?:implemented|supported)", re.IGNORECASE)
CRASH = re.compile(r"panicked at|RUST_BACKTRACE|stack overflow|memory allocation of")
RULE = re.compile(r"\bE_[A-Z_]+\b|§\d+(?:\.\d+)*")


class Delta:
    def __init__(self, path, a, b, mode):
        self.path, self.a, self.b, self.mode = path, a, b, mode

    def as_json(self):
        return {"path": self.path, "a": _clip(self.a), "b": _clip(self.b), "mode": self.mode}

    def __repr__(self):
        return "Delta(%s %s: %r != %r)" % (self.mode, self.path, _clip(self.a), _clip(self.b))


def _clip(v):
    text = json.dumps(v, sort_keys=True)
    return v if len(text) <= 400 else text[:400] + "...(clipped)"


class Diff:
    def __init__(self):
        self.hard, self.tolerated, self.relational = [], [], []


def reject_constant(name):
    raise ValueError("non-JSON constant %s" % name)


def parse_json(data):
    return json.loads(data, parse_constant=reject_constant)


def classify(code, stdout, stderr, timed_out):
    """Kind of one process outcome: json, diagnostic, unsupported, timeout, crash or badoutput."""
    if timed_out:
        return "timeout"
    if code is None or code < 0 or code not in (0, 1) or CRASH.search(stderr):
        return "crash"
    if code == 0:
        try:
            value = parse_json(stdout)
        except (ValueError, UnicodeDecodeError):
            return "badoutput"
        return "json" if isinstance(value, dict) else "badoutput"
    if stdout.strip():
        return "badoutput"
    return "unsupported" if UNSUPPORTED.search(stderr) else "diagnostic"


def rule_token(stderr):
    m = RULE.search(stderr)
    return m.group(0) if m else ""


def _num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _exact_scalar(a, b):
    if isinstance(a, bool) or isinstance(b, bool):
        return isinstance(a, bool) and isinstance(b, bool) and a == b
    if _num(a) and _num(b):
        if isinstance(a, float) and isinstance(b, float):
            return struct.pack("<d", a) == struct.pack("<d", b)
        return a == b
    return type(a) is type(b) and a == b


def _walk(a, b, path, d):
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b)):
            sub = "%s.%s" % (path, k) if path else k
            if k not in a or k not in b:
                d.hard.append(Delta(sub, a.get(k, "<absent>"), b.get(k, "<absent>"), "exact"))
            else:
                _walk(a[k], b[k], sub, d)
    elif isinstance(a, list) and isinstance(b, list):
        if len(a) != len(b):
            d.hard.append(Delta(path + ".length", len(a), len(b), "exact"))
        for i, (x, y) in enumerate(zip(a, b)):
            _walk(x, y, "%s[%d]" % (path, i), d)
    elif not _exact_scalar(a, b):
        d.hard.append(Delta(path, a, b, "exact"))


def _amplitude(a, b, path, d):
    """Compare two [re, im] pairs under the tolerance mode."""
    if not (isinstance(a, list) and isinstance(b, list) and len(a) == 2 and len(b) == 2
            and all(_num(x) for x in a + b)):
        d.hard.append(Delta(path, a, b, "tolerance"))
        return
    for i in range(2):
        if _exact_scalar(a[i], b[i]):
            continue
        sub = "%s[%d]" % (path, i)
        if abs(a[i] - b[i]) <= AMPLITUDE_TOLERANCE:
            d.tolerated.append(Delta(sub, a[i], b[i], "tolerance"))
        else:
            d.hard.append(Delta(sub, a[i], b[i], "tolerance"))


def _state(a, b, root, d):
    if not (isinstance(a, dict) and isinstance(b, dict)):
        _walk(a, b, root, d)
        return
    for k in sorted(set(a) | set(b)):
        sub = "%s.%s" % (root, k)
        if k in a and k in b:
            _amplitude(a[k], b[k], sub, d)
        else:
            present = a.get(k, b.get(k))
            if (isinstance(present, list) and len(present) == 2 and all(_num(x) for x in present)
                    and all(abs(x) <= AMPLITUDE_TOLERANCE for x in present)):
                d.tolerated.append(Delta(sub, a.get(k, "<absent>"), b.get(k, "<absent>"), "tolerance"))
            else:
                d.hard.append(Delta(sub, a.get(k, "<absent>"), b.get(k, "<absent>"), "tolerance"))


def compare_json(a, b):
    d = Diff()
    if not (isinstance(a, dict) and isinstance(b, dict)):
        _walk(a, b, "", d)
        return d
    for key in sorted(set(a) | set(b)):
        if key not in a or key not in b:
            d.hard.append(Delta(key, a.get(key, "<absent>"), b.get(key, "<absent>"), "exact"))
        elif key == "memory_digest":
            d.relational.append(key)
        elif key == "entry_memory_digest":
            pass
        elif key == "first_difference":
            if (a[key] is None) != (b[key] is None):
                d.hard.append(Delta(key, a[key], b[key], "nullness"))
        elif key in TOLERANT_ROOTS:
            _state(a[key], b[key], key, d)
        else:
            _walk(a[key], b[key], key, d)
    if "memory_digest" in a and "entry_memory_digest" in a and "memory_digest" in b and "entry_memory_digest" in b:
        ra = a["memory_digest"] == a["entry_memory_digest"]
        rb = b["memory_digest"] == b["entry_memory_digest"]
        if ra != rb:
            d.hard.append(Delta("memory_digest==entry_memory_digest", ra, rb, "relational"))
    return d


def collapse(path):
    """Group a diverging path into its top-level field for signatures (`$` paths stay whole)."""
    if path.startswith("$") or "==" in path:
        return path
    return re.split(r"[.\[]", path, maxsplit=1)[0]


# --- spec-oracle judging (published expectations, independent of pairwise comparison) ---

def subset_problems(expected, actual, path="$"):
    """JSON subset: objects field-wise (extras ignored), arrays exact length, numbers by binary64 value."""
    if isinstance(expected, dict):
        if not isinstance(actual, dict):
            return ["%s: expected an object, got %r" % (path, actual)]
        out = []
        for k, v in expected.items():
            if k not in actual:
                out.append("%s.%s: field missing" % (path, k))
            else:
                out.extend(subset_problems(v, actual[k], "%s.%s" % (path, k)))
        return out
    if isinstance(expected, list):
        if not isinstance(actual, list) or len(expected) != len(actual):
            return ["%s: expected array %r, got %r" % (path, _clip(expected), _clip(actual))]
        out = []
        for i, (x, y) in enumerate(zip(expected, actual)):
            out.extend(subset_problems(x, y, "%s[%d]" % (path, i)))
        return out
    if _num(expected) and _num(actual):
        return [] if float(expected) == float(actual) else ["%s: expected %r, got %r" % (path, expected, actual)]
    if type(expected) is not type(actual) or expected != actual:  # true is not 1: bool and int are distinct types
        return ["%s: expected %r, got %r" % (path, _clip(expected), _clip(actual))]
    return []


def dotted(value, path):
    for part in path.split("."):
        if not isinstance(value, dict) or part not in value:
            return False, None
        value = value[part]
    return True, value


def json_equal(a, b):
    d = Diff()
    _walk(a, b, "", d)
    return not d.hard


def judge(expect, code, stdout, stderr):
    """Problems of one outcome against an expectation dict (see engine.subset_expect and corpora)."""
    problems = []
    if code != expect["exit"]:
        problems.append("exit %r, expected %r" % (code, expect["exit"]))
    if expect.get("stdout_empty") and stdout.strip():
        problems.append("stdout not empty")
    if "stdout" in expect and stdout.decode("utf-8", "replace") != expect["stdout"]:
        problems.append("stdout bytes differ")
    wanted = expect.get("stderr_contains")
    if wanted:
        for w in [wanted] if isinstance(wanted, str) else wanted:
            if w not in stderr:
                problems.append("stderr lacks %r" % w)
    needs_json = any(k in expect for k in ("stdout_json", "exact_json", "equal_fields", "hash_view"))
    if not needs_json or code != expect["exit"]:
        return problems
    try:
        got = parse_json(stdout)
    except (ValueError, UnicodeDecodeError) as e:
        return problems + ["stdout is not JSON: %s" % e]
    if "stdout_json" in expect:
        problems.extend(subset_problems(expect["stdout_json"], got))
    if "exact_json" in expect:
        # Same field policy as pairwise comparison: digests stay opaque, first_difference is nullness.
        hard = compare_json(expect["exact_json"], got).hard
        if hard:
            problems.append("stdout JSON differs from expected: %s" % hard[:3])
    for path in expect.get("exact_at", ()):
        ok_e, want = dotted(expect.get("stdout_json"), path)
        ok_g, have = dotted(got, path)
        if not ok_e or not ok_g:
            problems.append("exact_at path %s missing" % path)
        elif not json_equal(want, have):
            problems.append("exact_at %s differs" % path)
    for a, b in expect.get("equal_fields", ()):
        ok_a, x = dotted(got, a)
        ok_b, y = dotted(got, b)
        if not (ok_a and ok_b and json_equal(x, y)):
            problems.append("fields %s and %s differ" % (a, b))
    if "hash_view" in expect:
        problems.extend(_hash_view_problems(expect["hash_view"], got))
    return problems


def _hash_view_problems(want, got):
    out = []
    if got.get("object_hash") != want["object"]:
        out.append("object hash differs")
    have = got.get("symbols") or {}
    if set(have) != set(want["symbols"]):
        out.append("symbol sets differ")
        return out
    for n, h in want["symbols"].items():
        if have[n].get("interface_hash") != h["interface"] or have[n].get("closure_hash") != h["closure"]:
            out.append("hashes of %s differ" % n)
    return out
