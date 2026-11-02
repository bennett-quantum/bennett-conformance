"""Black-box differential: this replay against a published checker binary.

Corpus: every published certificate, per-field certificate mutants, object mutants
with every hash rebound (only semantics can object), and synthetic objects covering
row families the published corpus lacks. Compared per exhaustive claim: outcome,
element count and covered fraction. A non-claim rejection by the binary is outside
this replay's scope and counted separately, never treated as agreement on a claim.

python3 -I -m checker_replay.differential --checker BIN --out DIR
"""
import argparse, collections, copy, json, os, pathlib, re, subprocess, sys
from . import hashes as H, jcs
from .objparse import parse_object
from .replay import check_certificate

VERSIONS = {k: "0.9" for k in ("cert", "coverage", "isa", "launch", "object", "sched", "target")}
HEX = re.compile(r'[0-9a-f]{64}')
MUTANT_ELEMENT_LIMIT = 4096


def norm_frac(f):
    return None if f is None else ("1" if f in ("1/1", "1") else f)


def compare_claim(r, b):
    """Return None on agreement, else a short class label. r/b are per-claim outcome dicts."""
    ro, bo = r.get("outcome"), b.get("outcome") if b else "absent"
    if ro != bo:
        return f"{ro}|{bo}"
    if ro in ("proof", "evidence"):
        if str(r.get("elements")) != str(b.get("elements")) and "covered_fraction" in r:
            return f"{ro}|{bo}|elements"
        if "covered_fraction" in r and norm_frac(r["covered_fraction"]) != norm_frac(b.get("covered_fraction")):
            return f"{ro}|{bo}|fraction"
    return None


def run_binary(binary, cert_path, obj_path, extra, workdir, timeout=120):
    cmd = [str(binary)] + ([str(cert_path), "--object", str(obj_path)] if cert_path else ["--from-object", str(obj_path)]) + list(extra)
    env = {k: v for k, v in os.environ.items() if k not in ("DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "BROWSER")}
    try:
        p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout, env=env, cwd=workdir)
    except subprocess.TimeoutExpired:
        return {"overall": "timeout"}, -1, ""
    try:
        return json.loads(p.stdout), p.returncode, p.stderr
    except ValueError:
        return {"overall": "crash", "stderr": p.stderr[-2000:]}, p.returncode, p.stderr


def rebind(text, cert=None):
    """Recompute interface hashes in the symbols line; rewrite old hashes in the certificate."""
    lines = text.split('\n')
    i = lines.index('section symbols')
    syms = json.loads(lines[i + 1])
    obj = parse_object(text)
    old = H.compute_hashes(parse_object(text))
    new = H.compute_hashes(obj)
    for s in syms.values():
        s["interface_hash"] = new[s["name"]][0]
    lines[i + 1] = jcs.canonical(syms).decode()
    out = '\n'.join(lines)
    return out, old, new


def strip_cert(text):
    """Object text without its certificate section (§9.2 deletes it from object_hash anyway)."""
    return re.sub(r'\n\nsection certificate\n[^\n]*', '', text)


def rebound_pair(old_text, new_body):
    """new_body is old_text with a function edit; give both a coherent certificate."""
    old_hashes = H.compute_hashes(parse_object(old_text))
    lines = old_text.split('\n')
    cert = json.loads(lines[lines.index('section certificate') + 1])
    body = strip_cert(new_body)
    new_text, _o, new_hashes = rebind(body)
    mapping = {}
    for name, (i_old, c_old) in old_hashes.items():
        i_new, c_new = H.compute_hashes(parse_object(new_text))[name]
        mapping[i_old] = i_new
        mapping[c_old] = c_new
    s = json.dumps(cert)
    s = HEX.sub(lambda m: mapping.get(m.group(0), m.group(0)), s)
    cert = json.loads(s)
    cert["bindings"]["object_hash"] = H.object_hash(new_text)
    return new_text, cert


OBJECT_EDITS = [
    ("cx_swap_operands", re.compile(r'^cx (\S+) (\S+)$', re.M), r'cx \2 \1'),
    ("ccx_swap_target", re.compile(r'^ccx (\S+) (\S+) (\S+)$', re.M), r'ccx \1 \3 \2'),
    ("swap_to_cx", re.compile(r'^swap (\S+) (\S+)$', re.M), r'cx \1 \2'),
    ("add_to_sub", re.compile(r'= lift\.add '), '= lift.sub '),
    ("sub_to_add", re.compile(r'= lift\.sub '), '= lift.add '),
    ("ult_to_ule", re.compile(r'lift\.icmp\.ult'), 'lift.icmp.ule'),
    ("select_swap", re.compile(r'= lift\.select (\S+) (\S+) (\S+)$', re.M), r'= lift.select \1 \3 \2'),
    ("qinplace_swap", re.compile(r'^qinplace @(\S+) @(\S+) ', re.M), r'qinplace @\2 @\1 '),
    ("x_extra", re.compile(r'^x (\S+)$', re.M), r'x \1\nx \1\nx \1'),  # equivalent: both must still accept
]


def object_mutants(text):
    out = []
    head = text.index('section functions')
    end = text.index('\n\nsection ', head)
    body = text[head:end]
    for tag, pat, rep in OBJECT_EDITS:
        m = pat.search(body)
        if not m:
            continue
        expanded = m.expand(rep)
        edited = body[:m.start()] + expanded + body[m.end():]
        out.append((f"object:{tag}", text[:head] + edited + text[end:]))
    return out


def claim_mutants(cl):
    """(tag, edit) certificate mutations for one exhaustive claim."""
    m = []

    def add(tag, fn):
        m.append((tag, fn))
    add("label_drop", lambda c: c.pop("label", None))
    add("label_evidence", lambda c: c.update(label="evidence"))
    add("label_proof", lambda c: c.update(label="proof"))
    add("label_invalid", lambda c: c.update(label="maybe"))
    add("unknown_field", lambda c: c.update(extra=1))
    add("subject_missing", lambda c: c.pop("subject"))
    add("expected_missing", lambda c: c.pop("expected"))
    add("subject_closure_unknown", lambda c: c["subject"].update(closure_hash="0" * 64))
    add("subject_closure_bad_hex", lambda c: c["subject"].update(closure_hash="xyz"))
    add("subject_variant_empty", lambda c: c["subject"].update(variant=""))
    add("subject_variant_other", lambda c: c["subject"].update(variant="other"))
    add("subject_inherited", lambda c: c["subject"].update(inherited_controls=1))
    add("subject_inherited_type", lambda c: c["subject"].update(inherited_controls="0"))
    add("subject_unknown_field", lambda c: c["subject"].update(x=1))
    add("subject_spurious_schedule", lambda c: c["subject"].update(schedule_hash="0" * 64))
    add("domain_missing", lambda c: c.pop("domain"))
    add("domain_regions_missing", lambda c: c["domain"].pop("regions"))
    add("domain_borrowed_beyond", lambda c: c["domain"]["borrowed"].append({"name": "zz", "width": 1, "lo": 0, "hi": 1}))
    add("domain_unknown_field", lambda c: c["domain"].update(x=[]))
    add("domain_region_unknown", lambda c: c["domain"]["regions"].append({"name": "zz", "cells": "all"}))
    bundles = cl["domain"]["bundles"]
    if bundles:
        add("bundle_name_unknown", lambda c: c["domain"]["bundles"][0].update(name="zz"))
        add("bundle_name_type", lambda c: c["domain"]["bundles"][0].update(name=7))
        add("bundle_width", lambda c: c["domain"]["bundles"][0].update(width=c["domain"]["bundles"][0]["width"] + 1))
        add("bundle_lo_gt_hi", lambda c: c["domain"]["bundles"][0].update(lo=1, hi=0))
        add("bundle_hi_overflow", lambda c: c["domain"]["bundles"][0].update(hi=1 << c["domain"]["bundles"][0]["width"]))
        add("bundle_negative", lambda c: c["domain"]["bundles"][0].update(lo=-1))
        add("bundle_duplicate", lambda c: c["domain"]["bundles"].append(dict(c["domain"]["bundles"][0])))
        add("bundle_decimal_string", lambda c: c["domain"]["bundles"][0].update(hi=str(c["domain"]["bundles"][0]["hi"])))
        add("bundle_float", lambda c: c["domain"]["bundles"][0].update(hi=float(c["domain"]["bundles"][0]["hi"])))
    exp = cl.get("expected")
    if isinstance(exp, dict) and "table" in exp:
        t = lambda c: c["expected"]["table"]

        def flip(field):
            def fn(c):
                r = t(c)[0][field]
                r["bundles"][0] = str(int(r["bundles"][0]) ^ 1)
            return fn
        add("table_drop_row", lambda c: t(c).pop())
        add("table_dup_row", lambda c: t(c).append(copy.deepcopy(t(c)[0])))
        add("table_swap_rows", lambda c: t(c).__setitem__(slice(0, 2), [t(c)[1], t(c)[0]]))
        add("table_flip_after", flip("after"))
        add("table_flip_before", flip("before"))
        add("table_flip_last_after", lambda c: t(c)[-1]["after"]["bundles"].__setitem__(-1, str(int(t(c)[-1]["after"]["bundles"][-1]) ^ 1)))
        add("table_noncanonical", lambda c: t(c)[0]["after"]["bundles"].__setitem__(0, "0" + t(c)[0]["after"]["bundles"][0]))
        add("table_number", lambda c: t(c)[0]["after"]["bundles"].__setitem__(0, int(t(c)[0]["after"]["bundles"][0])))
        add("table_out_of_range", lambda c: t(c)[0]["after"]["bundles"].__setitem__(0, "340282366920938463463374607431768211456" * 3))
        add("table_short", lambda c: t(c)[0]["after"]["bundles"].pop())
        add("table_long", lambda c: t(c)[0]["after"]["bundles"].append("0"))
        add("table_row_field", lambda c: t(c)[0].update(x=1))
        add("table_tuple_field", lambda c: t(c)[0]["after"].update(x=[]))
        add("table_type", lambda c: c["expected"].update(table={}))

        def slice_consistent(c):
            d0 = c["domain"]["bundles"][0]
            d0["hi"] = d0["lo"]
            pos = 0
            lo = d0["lo"]
            c["expected"]["table"] = [r for r in t(c) if int(r["before"]["bundles"][pos]) == lo]
            c["label"] = "evidence"
        add("slice_consistent", slice_consistent)

        def slice_proof(c):
            slice_consistent(c)
            c["label"] = "proof"
        add("slice_requests_proof", slice_proof)
    if exp != "inverse_pair":
        add("invocation_missing", lambda c: c.pop("invocation", None))
        add("invocation_budget_number", lambda c: c.setdefault("invocation", {"args": [], "budget": []}).update(budget=[9, 0, 0, 0]))
        add("invocation_budget_short", lambda c: c.setdefault("invocation", {"args": [], "budget": []}).update(budget=["9", "0", "0"]))
        add("invocation_budget_noncanon", lambda c: c.setdefault("invocation", {"args": [], "budget": []}).update(budget=["09", "0", "0", "0"]))
        add("invocation_args_extra", lambda c: c.setdefault("invocation", {"args": [], "budget": []})["args"].append("1"))
        add("invocation_unknown", lambda c: c.setdefault("invocation", {"args": [], "budget": []}).update(x=1))
        add("expected_oracle", lambda c: c.update(expected="oracle"))
        add("expected_program_self", lambda c: c.update(expected={"program": c["subject"]["closure_hash"]}))
        add("expected_program_unknown", lambda c: c.update(expected={"program": "0" * 64}))
        add("expected_both", lambda c: c.update(expected={"program": "0" * 64, "table": []}))
        add("expected_string", lambda c: c.update(expected="identity"))
        add("expected_inverse_pair_unindexed", lambda c: c.update(expected="inverse_pair"))
    else:
        add("inverse_invocation_present", lambda c: c.update(invocation={"args": [], "budget": ["9", "0", "0", "0"]}))
        add("inverse_slice_x", lambda c: c["domain"]["bundles"][0].update(hi=0))
        add("inverse_label_proof", lambda c: c.update(label="proof"))
        add("inverse_label_evidence", lambda c: c.update(label="evidence"))
        add("inverse_borrowed", lambda c: c["domain"]["borrowed"].append({"name": "b", "width": 1, "lo": 0, "hi": 1}))
    return m


def entry_mutants():
    """Mutations of the claim's inverse_pairs entry."""
    return [
        ("entry_swap_g_ginv", lambda e: e.update(g=e["ginv"], ginv=e["g"])),
        ("entry_ginv_is_g", lambda e: e.update(ginv=e["g"])),
        ("entry_budget_zero", lambda e: e.update(budget=["0", "0", "0", "0"])),
        ("entry_budget_short", lambda e: e.update(budget=e["budget"][:3])),
        ("entry_budget_heap", lambda e: e.update(budget=[e["budget"][0], "1", "0", "0"])),
        ("entry_budget_number", lambda e: e.update(budget=[int(x) for x in e["budget"]])),
        ("entry_budget_large", lambda e: e.update(budget=[str(int(e["budget"][0]) + 1000), "0", "0", "0"])),
        ("entry_arg_tokens_spurious", lambda e: e.update(arg_tokens={"zz": ["1", "0", "0", "0"]})),
        ("entry_unknown_field", lambda e: e.update(x=1)),
        ("entry_claim_oob", lambda e: e.update(claim=99)),
    ]


def domain_size(cl):
    n = 1
    d = cl["domain"]
    for e in d["bundles"] + d["borrowed"] + d.get("args", []):
        n *= e["hi"] - e["lo"] + 1
    return n if not d["regions"] else 1 << 60


class Run:
    def __init__(self, binary, out):
        self.binary, self.out = pathlib.Path(binary).resolve(), pathlib.Path(out).resolve()
        (self.out / "work").mkdir(parents=True, exist_ok=True)
        self.counts = collections.Counter()
        self.classes = {}
        self.seq = 0

    def compare(self, family, tag, text, cert, source, extra=(), target=None, schedule=None):
        self.seq += 1
        work = self.out / "work"
        obj_p, cert_p = work / "o.bisa", work / "c.json"
        obj_p.write_bytes(text.encode())
        cert_p.write_bytes(jcs.canonical(cert))
        self.counts["cases"] += 1
        try:
            mine = check_certificate(cert, parse_object(text), text, target=target, schedule=schedule)
        except Exception as e:  # a replay crash is itself a finding: never masked as agreement
            mine = {"overall": "crash", "regions": {}, "errors": [], "exception": repr(e)}
        theirs, code, stderr = run_binary(self.binary, cert_p, obj_p, extra, work)
        self.counts[f"binary_exit_{code}"] += 1
        found = []
        if mine.get("overall") == "crash" or theirs.get("overall") in ("crash", "timeout"):
            found.append(f"crash:{mine.get('overall')}|{theirs.get('overall')}")
        else:
            bregs = theirs.get("regions") or {}
            for key, rec in (mine.get("regions") or {}).items():
                for c in rec["claims"]:
                    if c.get("kind") != "exhaustive":
                        continue
                    self.counts["claims_compared"] += 1
                    claims = (bregs.get(key) or {}).get("claims") or []
                    b = claims[c["index"]] if c["index"] < len(claims) else None
                    label = compare_claim(c, b)
                    if label:
                        found.append(label)
                    else:
                        self.counts[f"agree_{c.get('outcome')}"] += 1
            self.document_level(mine, theirs, found)
        for label in sorted(set(found)):
            self.record(label, family, tag, text, cert, source, mine, theirs, extra)

    def document_level(self, mine, theirs, found):
        me_reject = bool(mine.get("errors"))
        them_reject = bool(theirs.get("errors"))
        if me_reject and not them_reject:
            found.append("doc-reject|doc-ok")
        elif them_reject and not me_reject:
            self.counts["binary_document_errors_out_of_replay_scope"] += 1

    def record(self, label, family, tag, text, cert, source, mine, theirs, extra):
        key = f"{family}:{tag}:{label}"
        self.counts["disagreements"] += 1
        cls = self.classes.setdefault(key, {"label": label, "family": family, "tag": tag, "count": 0, "witness": None, "sources": []})
        cls["count"] += 1
        if len(cls["sources"]) < 5:
            cls["sources"].append(source)
        if cls["witness"] is None:
            d = self.out / "witnesses" / re.sub(r'[^A-Za-z0-9_.-]+', '_', key)
            d.mkdir(parents=True, exist_ok=True)
            (d / "object.bisa").write_bytes(text.encode())
            (d / "certificate.json").write_text(json.dumps(cert, indent=0))
            (d / "replay.json").write_text(json.dumps(_claims_only(mine), indent=1, sort_keys=True))
            (d / "checker.json").write_text(json.dumps(_claims_only(theirs), indent=1, sort_keys=True))
            (d / "extra-args.json").write_text(json.dumps(list(map(str, extra))))
            cls["witness"] = str(d.relative_to(self.out))

    def published(self, name, text, cert, extra=(), target=None, schedule=None):
        self.compare("published", "as-is", text, cert, name, extra, target, schedule)

    def mutate_certificate(self, name, text, cert, extra=(), target=None, schedule=None):
        for key, rec in cert["regions"].items():
            for i, cl in enumerate(rec["claims"]):
                if cl.get("kind") != "exhaustive" or domain_size(cl) > MUTANT_ELEMENT_LIMIT:
                    continue
                for tag, fn in claim_mutants(cl):
                    mutant = copy.deepcopy(cert)
                    fn(mutant["regions"][key]["claims"][i])
                    self.compare("mutant", tag, text, mutant, f"{name}#{key[:8]}.{i}", extra, target, schedule)
                if cl.get("expected") == "inverse_pair":
                    pairs = rec["verification"]["inverse_pairs"]
                    for j, e in enumerate(pairs):
                        if e["claim"] != i:
                            continue
                        for tag, fn in entry_mutants():
                            mutant = copy.deepcopy(cert)
                            fn(mutant["regions"][key]["verification"]["inverse_pairs"][j])
                            self.compare("entry-mutant", tag, text, mutant, f"{name}#{key[:8]}.{i}", extra, target, schedule)
                # one sample of each claim shape per object is enough for these generic edits
                return

    def summary(self):
        return {"counts": dict(sorted(self.counts.items())),
                "classes": sorted(self.classes.values(), key=lambda c: (c["label"], c["family"], c["tag"]))}


def _claims_only(v):
    return {"overall": v.get("overall"), "errors": v.get("errors"), "exception": v.get("exception"),
            "stderr": v.get("stderr"),
            "regions": {k: {"errors": r.get("errors"), "claims": r.get("claims")} for k, r in (v.get("regions") or {}).items()}}


def published_objects(ws):
    """Every tracked object embedding a certificate, across the workspace's repositories."""
    for repo in sorted(p for p in ws.iterdir() if (p / ".git").exists()):
        names = subprocess.run(["git", "-C", str(repo), "grep", "-lE", "^section certificate$", "--", "*.bisa"],
                               capture_output=True, text=True).stdout.split("\n")
        for n in names:
            if n:
                yield repo / n


def main(argv=None):
    from .synthetic import synthetic_cases
    ap = argparse.ArgumentParser()
    ap.add_argument("--checker", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--workspace", default=str(pathlib.Path(__file__).resolve().parents[3]))
    a = ap.parse_args(argv)
    ws = pathlib.Path(a.workspace)
    run = Run(a.checker, a.out)
    fx = ws / "bennett-isa-spec/conformance/fixtures/controlled-action-09"
    controlled = {"target": json.loads((fx / "target.json").read_text()), "schedule": json.loads((fx / "schedule.json").read_text())}
    cextra = ["--target", fx / "target.json", "--schedule", fx / "schedule.json"]
    ctext = (fx / "controlled.bisa").read_bytes().decode()
    ccert = json.loads((fx / "certificate.json").read_text())
    run.published("controlled-action-09/certificate.json", ctext, ccert, cextra, **controlled)
    run.mutate_certificate("controlled-action-09", ctext, ccert, cextra, **controlled)
    mutated_per_shape = collections.Counter()
    for path in published_objects(ws):
        text = path.read_bytes().decode()
        lines = text.split('\n')
        try:
            cert = json.loads(lines[lines.index('section certificate') + 1])
        except (ValueError, IndexError):
            run.counts["published_unparseable"] += 1
            continue
        rel = str(path.relative_to(ws))
        run.counts["published_objects"] += 1
        try:
            parse_object(text)
        except Exception as e:
            run.counts["published_replay_parse_error"] += 1
            print(f"parse error {rel}: {e!r}", file=sys.stderr)
            continue
        run.published(rel, text, cert)
        if not text.startswith("bennett-object isa=0.9"):
            continue
        if any(c.get("kind") == "exhaustive" for r in cert.get("regions", {}).values() for c in r.get("claims", [])):
            run.mutate_certificate(rel, text, cert)
            for tag, edited in object_mutants(text):
                try:
                    new_text, new_cert = rebound_pair(text, edited)
                except Exception as e:
                    run.counts["object_mutant_build_error"] += 1
                    print(f"object mutant {tag} of {rel}: {e!r}", file=sys.stderr)
                    continue
                run.compare("object-mutant", tag, new_text, new_cert, rel)
    for name, text, cert in synthetic_cases():
        run.compare("synthetic", name, text, cert, name)
    s = run.summary()
    (pathlib.Path(a.out) / "summary.json").write_text(json.dumps(s, indent=1, sort_keys=True))
    print(json.dumps(s["counts"], sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
