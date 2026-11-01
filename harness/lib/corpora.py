"""Corpora as data: ISA conformance vectors, the neutral suite, VM differential fixtures, ladder objects.

Each loader yields engine.Case values and reports anything it does not drive through runner.skip.
"""
import glob
import json
import os

import drivers
import engine
import objfile

HERE = os.path.dirname(os.path.abspath(__file__))
HARNESS = os.path.dirname(HERE)


def _read(path):
    with open(path, encoding="utf-8", newline="") as f:
        return f.read()


def isa_vectors(workspace, runner):
    """bennett-isa-spec/conformance/cases: spec-derived expectations (§15), judged per executor."""
    corpus = "isa-vectors"
    root = os.path.join(workspace, "bennett-isa-spec", "conformance", "cases")
    paths = sorted(glob.glob(os.path.join(root, "*", "*", "case.json")))
    if not paths:
        raise ValueError("no ISA conformance vectors under %s" % root)
    for path in paths:
        d = os.path.dirname(path)
        with open(path, encoding="utf-8") as f:
            c = json.load(f)
        if "command" not in c:
            un = c.get("unavailable") or {}
            runner.skip(corpus, c["id"], "no CLI probe: %s" % _reason(un))
            continue
        obj = os.path.normpath(os.path.join(d, c["object"])) if "object" in c else None
        argv = ["$OBJECT" if a == "{object}" else a.replace("{dir}", d) for a in c["command"]]
        exp = c["expect"]
        expect = {"exit": exp["exit"]}
        for src, dst in (("stdout_json", "stdout_json"), ("stdout_json_exact_at", "exact_at"),
                         ("stdout_json_equal_fields", "equal_fields"), ("stdout_empty", "stdout_empty"),
                         ("stderr_contains", "stderr_contains")):
            if src in exp:
                expect[dst] = exp[src]
        alt = None
        un = c.get("unavailable")
        if un and "expect" in un:
            alt = {"exit": un["expect"]["exit"]} if "exit" in un["expect"] else {"exit": 1}
            for k in ("stdout_empty", "stderr_contains"):
                if k in un["expect"]:
                    alt[k] = un["expect"][k]
        yield engine.Case(corpus, c["id"], argv, object_path=obj, cwd=d, expect=expect, alt_expect=alt,
                          unavailable=bool(un))
    extra = sorted(glob.glob(os.path.join(workspace, "bennett-isa-spec", "conformance", "fixtures", "*", "*")))
    for path in extra:
        runner.skip(corpus, os.path.basename(path),
                    "non-CLI fixture data (crate-level contract test, no executor command)")


def _reason(un):
    if "deliverable" in un:
        return "staged deliverable %s" % un["deliverable"]
    return "scenario only"


def neutral_suite(repo, runner):
    """bennett-conformance/suite/cases: exact expectations recorded from the accepted reference snapshot."""
    corpus = "neutral-suite"
    paths = sorted(glob.glob(os.path.join(repo, "suite", "cases", "*", "expect.json")))
    if not paths:
        raise ValueError("no neutral suite cases")
    for path in paths:
        d = os.path.dirname(path)
        name = os.path.basename(d)
        obj = os.path.join(d, "object.bisa")
        with open(path, encoding="utf-8") as f:
            e = json.load(f)
        if "hashes" in e:
            yield engine.Case(corpus, name + "/hash", ["hash", "$OBJECT"], object_path=obj, cwd=d,
                              expect={"exit": 0, "hash_view": e["hashes"]})
        chk = e.get("check")
        if chk == "ok":
            yield engine.Case(corpus, name + "/check", ["check", "$OBJECT"], object_path=obj, cwd=d,
                              expect={"exit": 0, "stdout_json": {"status": "ok"}})
        elif chk is not None:
            yield engine.Case(corpus, name + "/check", ["check", "$OBJECT"], object_path=obj, cwd=d,
                              expect={"exit": chk["exit"], "stderr_contains": chk["stderr_contains"]})
        for i, r in enumerate(e.get("runs", ())):
            argv = [a.replace("$OBJECT", "\0").replace("$CASE", d) for a in r["cmd"]]
            argv = ["$OBJECT" if a == "\0" else a.replace("\0", obj) for a in argv]
            expect = {"exit": r["exit"]}
            if "stdout_json" in r:
                expect["exact_json"] = r["stdout_json"]
            else:
                expect["stdout"] = r["stdout"]
            if "stderr_contains" in r:
                expect["stderr_contains"] = r["stderr_contains"]
            yield engine.Case(corpus, "%s/run%d" % (name, i), argv, object_path=obj, cwd=d, expect=expect)


def vm_differential(workspace, runner):
    """Invocations literally present in bennett-vm's differential tests (extracted data), over published fixtures."""
    corpus = "vm-differential"
    data_path = os.path.join(HARNESS, "corpora", "vm-differential.json")
    with open(data_path, encoding="utf-8") as f:
        data = json.load(f)
    fixtures = os.path.join(workspace, "bennett-isa-spec", "reference-interpreter", "tests", "fixtures")
    for row in data["skipped"]:
        runner.skip(corpus, "line %d" % row["line"], row["reason"])
    for i, row in enumerate(data["invocations"]):
        obj = os.path.join(fixtures, row["fixture"])
        if not os.path.exists(obj):
            raise ValueError("VM differential invocation names missing fixture %s" % obj)
        argv = ["$OBJECT" if a == "$FIXTURE" else a for a in row["argv"]]
        yield engine.Case(corpus, "line%d-%d" % (row["line"], i), argv, object_path=obj, cwd=fixtures)


def _object_corpus(corpus, paths, runner, root):
    for path in paths:
        rel = os.path.relpath(path, root)
        try:
            o = objfile.parse(_read(path))
        except objfile.ObjectError as e:
            runner.skip(corpus, rel, "harness parser refused object: %s" % e)
            continue
        yield from drivers.for_object(corpus, rel, runner, object_path=path, o=o, cwd=os.path.dirname(path))


def fixture_objects(workspace, runner):
    """Every published reference fixture and conformance object, driven from its signatures."""
    fixtures = os.path.join(workspace, "bennett-isa-spec", "reference-interpreter", "tests", "fixtures")
    objects = os.path.join(workspace, "bennett-isa-spec", "conformance", "cases", "objects")
    paths = sorted(glob.glob(os.path.join(fixtures, "**", "*.bisa"), recursive=True))
    paths += sorted(glob.glob(os.path.join(objects, "*.bisa")))
    yield from _object_corpus("fixture-objects", paths, runner, workspace)


def ladder_evidence(repo, runner):
    """Compiled objects the ladder gates produced or use (committed evidence and fixtures)."""
    ladder = os.path.join(repo, "ladder")
    paths = []
    for pat in ("**/*.bennett", "**/*.bisa"):
        paths += glob.glob(os.path.join(ladder, pat), recursive=True)
    if not paths:
        raise ValueError("no ladder objects")
    yield from _object_corpus("ladder-evidence", sorted(paths), runner, repo)


LOADERS = ("isa-vectors", "neutral-suite", "vm-differential", "fixture-objects", "ladder-evidence")


def load(name, workspace, repo, runner):
    if name == "isa-vectors":
        return isa_vectors(workspace, runner)
    if name == "neutral-suite":
        return neutral_suite(repo, runner)
    if name == "vm-differential":
        return vm_differential(workspace, runner)
    if name == "fixture-objects":
        return fixture_objects(workspace, runner)
    if name == "ladder-evidence":
        return ladder_evidence(repo, runner)
    raise ValueError("unknown corpus %r (known: %s)" % (name, ", ".join(LOADERS)))
