#!/usr/bin/env python3
"""A fake ISA executor for the harness self-tests. Behaviour is chosen by FAKE_MODE.

base      {"entry","value":1,"state":{"0":[0.5,0.0]}}, loads objects and rejects bad stored hashes
buggy     as base, but value=2 when the object has a `xor` line (a planted divergence)
amp       as base, amplitude perturbed by 1e-15 (within tolerance, not exact)
unsupported  exit 1 "unsupported: milestone 9"
crash     exit 101 with a panic message
slow      sleeps beyond the timeout
reject    exit 1 with a diagnostic
garbage   exit 0 with non-JSON stdout
tape      as base and honours --tape-out; FAKE_TAPE overrides the tape body
FAKE_VALUE overrides the base integer result for oracle tests.
"""
import json
import os
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "lib"))
import objfile  # noqa: E402
import identity  # noqa: E402

mode = os.environ.get("FAKE_MODE", "base")
argv = sys.argv[1:]
if mode == "unsupported":
    print("unsupported: milestone 9", file=sys.stderr)
    sys.exit(1)
if mode == "crash":
    print("thread 'main' panicked at src/x.rs:1", file=sys.stderr)
    sys.exit(101)
if mode == "slow":
    time.sleep(30)
    sys.exit(0)
if mode == "reject":
    print("E_ARG: §10 rejected", file=sys.stderr)
    sys.exit(1)
if mode == "garbage":
    print("not json")
    sys.exit(0)
path = argv[1]
with open(path, encoding="utf-8", newline="") as f:
    text = f.read()
try:
    o = objfile.parse(text)
    bad = identity.stored_mismatches(o)
except (objfile.ObjectError, identity.IdentityError, KeyError) as e:
    print("E_LOAD: %s" % e, file=sys.stderr)
    sys.exit(1)
if bad:
    print("E_HASH: §9.2 %s" % bad[0], file=sys.stderr)
    sys.exit(1)
value = int(os.environ.get("FAKE_VALUE", "1"))
if mode == "buggy" and any(line.endswith(" xor a b") or " = xor " in line
                           for f in o.functions for line in f.lines):
    value = 2
amp = 0.5 + (1e-15 if mode == "amp" else 0.0)
out = {"entry": argv[2] if len(argv) > 2 else None, "value": value, "state": {"0": [amp, 0.0]},
       "memory_digest": ("%064x" % (7 if mode == "amp" else 9))}
if "--tape-out" in argv:
    with open(argv[argv.index("--tape-out") + 1], "w", encoding="utf-8") as f:
        f.write(os.environ.get("FAKE_TAPE", "[]") + "\n")
print(json.dumps(out, sort_keys=True, separators=(",", ":")))
