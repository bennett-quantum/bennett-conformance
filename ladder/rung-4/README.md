# Rung 4: **Grover over SQLite**

Acceptance test (North Star section 8):

> SQLite compiled unmodified to the coherent tier under a region budget, with an in-memory VFS over quantum-addressed memory. Database of N rows fixed classically before entry; the predicate is a real SQL query parameterised by a `rowid` bound from the superposed register; the oracle bit is the query's match for that row. (a) Oracle verified by §5.1 part 1 over all rows and all target values for N ≤ 2¹⁰, fault rate zero. (b) End-to-end Grover by state-vector simulation with branch enumeration at N ≤ 2⁸. (c) A resource estimate for N = 2²⁰ under a named target, reporting the budget and the padded cost. Nobody rewrote SQLite.

## Gate

`./gate.sh` runs the whole acceptance on both LLVM majors and exits 0 only if every row passes; red now
is expected. [DESIGN.md](DESIGN.md) says what each clause means and which shortcuts are forbidden;
[RESULTS.md](RESULTS.md) records the measured frontier and blockers. Glue tests:
`TMPDIR=<repo>/.tmp/rung-4/tmp python3 check_gate.py`. `fixtures/fetch-sqlite.sh` fetches the pinned
amalgamation into the cache if `~/.local/share/bennett-sqlite/` does not have it.
