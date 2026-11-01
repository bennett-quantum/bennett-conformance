# Handoff: bennett-conformance
State (2026-10-31): clause graph + statement layer (`clauses/`; ISA-0.8, ISA-0.9-draft and DLT statement
views, dialect and .dot outputs generated on demand; 113 tests); neutral suite (`suite/`, 16 cases re-imported
from the accepted reference-interpreter snapshot ad84e75, green); six structured proofs (`proofs/`, INV-1..6;
statement-level strong coverage 169/541 on ISA 0.8). Gate: `scripts/pre-push` (cargo + clause tests + suite
tests + neutral suite against `BENNETT_ISA_BINARY` or the review worktree build).
Next: proofs are consumed by council rounds as `--context`; a new proof is written when a council finding fits
no stated theorem. Graphviz for SVG and a Go toolchain for af ledgers remain optional.
Orchestrator state: bennett/HANDOFF.md.

## Ladder rungs — 2026-11-01T14:09:00Z

### Rung 1 — real gate remains blocked

Implemented `ladder/rung-1/gate.sh` and ordinary marker-free C/Rust/Julia
fixtures, with pinned dual-major plugin/runtime/arith and detached release VM
builds. Final full invocation `.tmp/rung-1/20261101T135131.872696Z/` exits 1.
All six O0/O2 native oracles agree over 131,840 inputs per language/major. All
five C functions pass every source input on both executors and both majors,
with zero faults and byte-exact reversal. Four C functions also have runnable
materialised native comparisons; lookup's fixed global address 4096 cannot run
on this host. Rust/23's sat_add, popcount and sign_abs pass their complete input
products; gcd/lookup refuse residual personality attributes. No full-D acceptance
is claimed: D also contains every target/fault value and clean workspace, and
all full-D rows remain blocked on the corpus prerequisites.

Ordered blockers: conformance `bennett-conformance-oc2` (compatible LLVM18 Rust/
Julia frontend publications); plugin `bp-xb8` (N4/N28 personality), `bp-zeg`
(Julia implicit gcstack entry state), `bp-75y` (published native/device layout
option). Rung acceptance bead `bennett-conformance-px1` is blocked. Exact
contracts, instructions, commands and completion criteria: rung-1/blockers.json;
full stage/per-function tables, hashes and compact evidence: rung-1/RESULTS.md.
No sibling runtime/reference/VM defect is asserted beyond the measured frontier.

Rung-1 eight checks, rung-2 five driver checks and rung-3 eight checks pass.
Rung-2/rung-3/common source is unchanged. Only sibling Beads exports were
committed by this task; concurrent sibling source work is not part of the run.
The final clean-worktree `scripts/pre-push` result is reported at handback. No
push or Dolt remote sync. Consumer discovery budget/native-harness mistakes
were fixed before the final full run; none was misfiled as an upstream defect.

### Rung 3 — previous recorded state


Rung 3 implementation: `ladder/rung-3/gate.sh`, pinned dual-major builds and real
ordinary musl `hello.c`; no freestanding or host-libc comparison fallback. Recorded
run `.tmp/rung-3/20261101T125405.461872Z/` exits 1 on both LLVM 18/23. Full table,
exact commands, hashes and compact durable evidence: `ladder/rung-3/RESULTS.md`.
The first corpus blocker is brt-1ft (musl stdio.h/sysroot/libc publication). Ordered
prerequisites: brt-cgb (console runtime), bp-ivy (R-mode integration),
bennett-driver-cyg (pipeline/native comparison), b-isa-spec-221 (ISA-0.8 typed R
fault dependency; ISA 0.9 published alongside), b-isa-spec-04d and bennett-vm-u8m
(saved-tape consumer interfaces). `blockers.json` contains owners, quoted spec
sentences, exact failed commands and completion criteria. Conformance bead
bennett-conformance-7bx remains blocked for full rung acceptance.

What passes: both plugin builds, frozen driver/VM builds, published device/reset
controls on both executors. What is still unproved: every musl hello-world native,
production, forward/reset and tape-replay row. Controls cannot satisfy those rows.
Runtimes/plugin/driver/ISA-spec/VM received beads-only commits, no sibling source
edits, no push or Dolt sync. Driver beads were initialised with its Dolt origin
removed; final driver diff contains only `.beads` files.

Only rung-2 command/error/hash/git helpers moved verbatim to `ladder/common/`;
every rung-2 function AST is unchanged and its five driver checks pass. Rung-3's
eight tests, checker two tests, 113 clause tests, 23 suite tests and 16 neutral
cases pass (release reference binary selected). Existing rung-2 RESULTS.md remains
the acceptance record for that rung; no new hashtable corpus run was claimed.

Next: owners implement the ordered upstream prerequisites, then rerun the full
gate with explicit revised pins and real musl publications. Preserve the full
North Star scope and explicit contract migration; never interpret known blockers
as passes. Raw evidence remains in `.tmp`, compact evidence is committed.
