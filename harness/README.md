# harness

Two differentials live here.

**Executor differential (built).** `bdiff.py` runs ISA objects through any set of executors as black boxes, over
corpora and seeded generators, and reports divergences with minimised witnesses and every skip with a count.
Usage, field modes and the recorded run: [docs/harness.md](../docs/harness.md). Witnesses: `witnesses/`. Runs: `runs/`.

**Native differential (not built, bead bennett-conformance-06p).** Run each program natively and through the compiled
pipeline on the same inputs, comparing results, with instrumented native builds (UBSan, ASan, MSan) as classifiers that
flag inputs outside the defined domain D (North Star 5.1 part 1, 3.2). Flagged inputs are reported, never silently dropped.
