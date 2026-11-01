#!/usr/bin/env python3
"""Differential harness over ISA executors (see docs/harness.md). Run: python3 -I harness/bdiff.py ..."""
import argparse
import json
import os
import re
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "lib"))
import corpora  # noqa: E402
import engine  # noqa: E402
import executors  # noqa: E402
import generate  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--exec", dest="execs", action="append", required=True, metavar="NAME=PATH[,ENV=VALUE...]",
                    help="an executor; at least two")
    ap.add_argument("--pair", action="append", default=[], metavar="A:B", help="compare only these pairs (default all)")
    ap.add_argument("--corpus", action="append", default=[], help="corpus name or 'all': " + ", ".join(corpora.LOADERS))
    ap.add_argument("--gen", action="append", default=[], help="generator class or 'all': " + ", ".join(generate.CLASSES))
    ap.add_argument("--seed", type=int, default=1, help="first seed of the generator sequence")
    ap.add_argument("--wall-seconds", type=float, default=60.0, help="wall bound per generator class")
    ap.add_argument("--count", type=int, default=0, help="programs per class (0: until the wall bound)")
    ap.add_argument("--timeout", type=float, default=60.0, help="seconds per executor process")
    ap.add_argument("--jobs", type=int, default=4)
    ap.add_argument("--shrink-seconds", type=float, default=30.0)
    ap.add_argument("--witnesses-per-signature", type=int, default=2)
    ap.add_argument("--witness-dir", default=os.path.join(HERE, "witnesses"))
    ap.add_argument("--workspace", default=os.path.abspath(os.path.join(HERE, "..", "..")))
    ap.add_argument("--summary", help="write the summary JSON here")
    args = ap.parse_args()

    execs = [executors.parse_spec(s, args.timeout) for s in args.execs]
    names = {e.name for e in execs}
    pairs = []
    for p in args.pair:
        a, _, b = p.partition(":")
        if a not in names or b not in names:
            ap.error("pair %s names an unknown executor" % p)
        pairs.append((a, b))
    if not args.corpus and not args.gen:
        ap.error("nothing to do: give --corpus and/or --gen")
    audit = {e.name: re.compile(r"JIT_AUDIT native_runs=[1-9]") for e in execs
             if e.env.get("BENNETT_JIT_AUDIT") == "1"}
    root = os.environ.get("TMPDIR")
    if not root or not root.startswith(args.workspace):
        ap.error("TMPDIR must be set under the workspace (no /tmp artefacts)")
    with tempfile.TemporaryDirectory(dir=root, prefix="bdiff-") as work:
        r = engine.Runner(execs, work, witness_dir=args.witness_dir, shrink_seconds=args.shrink_seconds,
                          witnesses_per_signature=args.witnesses_per_signature, pairs=pairs or None, audit=audit)
        repo = os.path.dirname(HERE)
        meta = {"generators": {}}
        for name in (corpora.LOADERS if "all" in args.corpus else args.corpus):
            started = time.monotonic()
            r.run_all(corpora.load(name, args.workspace, repo, r), jobs=args.jobs)
            print("corpus %s: %d cases in %.1fs" % (name, r.corpora[name]["cases"], time.monotonic() - started), flush=True)
        for cls in (generate.CLASSES if "all" in args.gen else args.gen):
            info = generate.run_class(r, cls, args.seed, args.wall_seconds, args.count, args.jobs)
            meta["generators"][cls] = info
            print("generator %s: %s" % (cls, info), flush=True)
        s = r.summary()
        s.update(meta)
        status = r.exit_status()
        s["exit_status"] = status
    if args.summary:
        engine.dump(s, args.summary)
    print(json.dumps({"totals": s["totals"], "divergence_signatures": len(s["divergences"]), "exit_status": status}, sort_keys=True))
    for row in s["divergences"]:
        print("DIVERGE %s x%d: %s [%s] %s" % (row["pair"], row["count"], row["signature"], row["first_case"],
                                              row["witnesses"][:1]))
    for row in s["oracle_failures"][:40]:
        print("ORACLE-FAIL %s %s/%s: %s" % (row["executor"], row["corpus"], row["case"], row["problem"][:160]))
    if len(s["oracle_failures"]) > 40:
        print("... %d oracle failures in total (see the summary JSON)" % len(s["oracle_failures"]))
    return status


if __name__ == "__main__":
    sys.exit(main())
