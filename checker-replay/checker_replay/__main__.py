"""CLI: python3 -I -m checker_replay CERT --object OBJ [--target T] [--schedule S] [--max-elements N]
       python3 -I -m checker_replay --from-object OBJ [--target T] [--schedule S]

One canonical JSON verdict on stdout. Exit: 0 every claim replayed; 1 a rejection;
2 something unchecked, no rejection; 3 usage or input error.
"""
import argparse, json, sys
from . import jcs
from .objparse import parse_object
from .replay import check_certificate, DEFAULT_MAX_ELEMENTS


def _no_dup(pairs):
    d = {}
    for k, v in pairs:
        if k in d:
            raise ValueError(f"duplicate JSON key {k!r}")
        d[k] = v
    return d


def _load(path):
    with open(path, encoding='utf-8') as f:
        return json.load(f, object_pairs_hook=_no_dup)


def main(argv=None):
    ap = argparse.ArgumentParser(prog="checker_replay", exit_on_error=False)
    ap.add_argument("certificate", nargs="?")
    ap.add_argument("--object")
    ap.add_argument("--from-object")
    ap.add_argument("--target")
    ap.add_argument("--schedule")
    ap.add_argument("--max-elements", type=int, default=DEFAULT_MAX_ELEMENTS)
    try:
        a = ap.parse_args(argv)
        path = a.from_object or a.object
        if path is None or (a.from_object and a.certificate) or not (a.from_object or a.certificate):
            raise ValueError("give CERT --object OBJ, or --from-object OBJ")
        with open(path, encoding='utf-8', newline='') as f:
            text = f.read()
        obj = parse_object(text)
        cert = obj.certificate if a.from_object else _load(a.certificate)
        if cert is None:
            raise ValueError(f"{path} embeds no certificate")
        target = _load(a.target) if a.target else None
        schedule = _load(a.schedule) if a.schedule else None
    except (OSError, ValueError, argparse.ArgumentError) as e:
        print(f"checker_replay: {e}", file=sys.stderr)
        return 3
    v = check_certificate(cert, obj, text, target=target, schedule=schedule, max_elements=a.max_elements)
    sys.stdout.write(jcs.canonical(v).decode() + "\n")
    return {"reject": 1, "unchecked": 2, "replayed": 0}[v["overall"]]


if __name__ == "__main__":
    sys.exit(main())
