# Rung 4: **Grover over SQLite**

Acceptance test (North Star section 8):

> SQLite compiled unmodified to the coherent tier under a region budget, with an in-memory VFS over quantum-addressed memory. Database of N rows fixed classically before entry; the predicate is a real SQL query parameterised by a `rowid` bound from the superposed register; the oracle bit is the query's match for that row. (a) Oracle verified by §5.1 part 1 over all rows and all target values for N ≤ 2¹⁰, fault rate zero. (b) End-to-end Grover by state-vector simulation with branch enumeration at N ≤ 2⁸. (c) A resource estimate for N = 2²⁰ under a named target, reporting the budget and the padded cost. Nobody rewrote SQLite.

## Gate

`./gate.sh` runs the whole acceptance on both LLVM majors and exits 0 only if every row passes; red now
is expected. [DESIGN.md](DESIGN.md) says what each clause means and which shortcuts are forbidden;
[RESULTS.md](RESULTS.md) records the measured frontier and blockers. Glue tests:
`TMPDIR=<repo>/.tmp/rung-4/tmp python3 check_gate.py`. `fixtures/fetch-sqlite.sh` fetches the pinned
amalgamation into the cache if `~/.local/share/bennett-sqlite/` does not have it.

`./gate.sh --diagnostics-only` runs builds, fixture checks, the independent
predicate and the published reduced-control diagnostics. It excludes SQLite
source/archive checks, the full host/compiler/executor pipeline and clauses (a), (b) and (c). Every
excluded row remains in the report as `NOT RUN`, with the selection recorded
in the manifest. This mode cannot return full-rung acceptance even when every
selected row passes. Use it to inspect prerequisites while the full SQLite
analysis memory blocker is open. `python3 check_selection.py` verifies that
this selection cannot call the excluded methods and that the default remains
the complete gate.


All sibling inputs default to [pins.json](pins.json). The selectors
`--plugin-rev`, `--vm-rev`, `--runtimes-rev`, `--driver-rev`, `--estimate-rev`,
`--targets-rev`, `--isa-spec-rev`, `--dialect-spec-rev`, `--conformance-rev`,
and `--arith-rev` accept a commit or ref and resolve it once to a clean detached
worktree. The report records the resolved commits and archive hashes. Unknown
revisions and dirty caches fail loudly. `BENNETT_GATE_WORKSPACE` can select the
workspace containing the source repositories, as in rungs 1–2.

The reference interpreter is built offline in release mode from the ISA pin in
the canonical `bennett-isa-spec` repository. `--reference` selects an explicit
binary and conflicts with `--isa-spec-rev`. Production requests the selected
plugin's declared dialect and ISA versions. An explicit plugin revision follows
its published arithmetic dependency unless `--arith-rev` is also supplied.
Arithmetic must match both of that plugin's bitcode digest pins. With no revision
flags, every input comes from pins.json, including arithmetic `0dea42d`, whose
two bitcode files match plugin `3a856a7`.

Plugin configure/build receive `BENNETT_GATE_LAYOUT` with frozen build
dependencies from the plugin's own gate.toml (including repository aliases).
Historical plugins without gate.toml receive the selected arithmetic publication
in that layout. Production uses `BENNETT_ARITH_PINNED_ROOT` pointing at the
selected frozen publication and clears `BENNETT_ARITH_BITCODE`, which would
bypass the plugin's digest check. No live distribution or review checkout
supplies a default input. The local revision helper can be unified with rungs
1–2 after merge (P3 `bennett-conformance-t6a`).

Revision regressions, including read-only main-health `ladder_argv` compatibility:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 ladder/rung-4/check_revisions.py -v
```

Full runs execute detached in a systemd user unit named
`bennett-ladder-rung4-<UTC time>`. Gates in different repositories may run concurrently;
never run two gates for this repository at once. Admit a run only when available memory
is at least its recorded peak RSS plus 6 GB and load is below half the hardware threads. On
`tobias-ThinkPad-X1-Carbon-Gen-11`, use a 12000-second outer hang guard (machine
factor 1), with exit 124 reported as UNKNOWN. This exceeds ten times the
1109.026 summed command wall seconds in the recorded 2026-11-01 report
`evidence/20261101T221646.155856Z/report.json`; summed durations conservatively
bound elapsed time because LLVM-major workers overlap. Record outer wall/user/
system CPU seconds alongside command receipts. Existing per-command timeouts
are unchanged pending the conformance time-policy sweep. The shell launcher
preserves an explicit TMPDIR; headless job runs use a worktree-local directory.


The reduced measured-control diagnostic uses the selected ISA revision's published
`reference-interpreter/tests/fixtures/registers.bisa`, entry `measurement`, via
`run-program --draws`. It checks both measured Bell basis states and records the
actual measurement results. It does not require the pending lossless state export.
The required Grover amplitude rows still require the pre-measurement readout in
DESIGN.md. An explicit reference binary still uses the ISA pin for this fixture.
