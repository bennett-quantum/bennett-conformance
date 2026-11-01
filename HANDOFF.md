# Handoff: bennett-conformance
Updated 2026-11-01 (see `git log` for the last commit).

## State
- `clauses/`: clause graph + statement layer for ISA 0.8, ISA 0.9 and DLT 0.2.10 (tracked). A DLT 0.2.11 draft
  view is untracked (reported to the orchestrator, not deleted). Dialect and .dot outputs are generated on demand.
- `suite/`: implementation-neutral conformance cases, re-imported from the accepted reference-interpreter snapshot.
- `proofs/`: six structured proofs INV-1..6 (`proofs/INDEX.md`).
- Gate: `scripts/pre-push` (cargo + clause tests + suite tests + neutral suite against `BENNETT_ISA_BINARY` or the
  review worktree build). Run it for current counts; none are recorded here.
- `SPEC_VERSIONS.toml` still reads `status = "setup"`, isa 0.8, dialect 0.2.10; nothing reads it.

## Ladder
- **Rung 1: blocked.** Gate `ladder/rung-1/gate.sh`, dual-major. Source-input checks pass where the frontend IR is
  applicable; full-D acceptance is not claimed. Ordered blockers: `bennett-conformance-oc2` is closed (frontend-major
  matrix ruling, §19.8); open: plugin `bp-xb8`, `bp-zeg`, `bp-75y`. Acceptance bead `bennett-conformance-px1` is
  blocked. Contracts and commands: `rung-1/blockers.json`; tables and hashes: `rung-1/RESULTS.md`.
- **Rung 2: complete** (hashtable, both executors, both majors). Record: `rung-2/RESULTS.md`.
- **Rung 3: blocked.** First blocker brt-1ft (musl sysroot publication); ordered prerequisites and quoted
  sentences in `rung-3/blockers.json`; results in `rung-3/RESULTS.md`. Bead `bennett-conformance-7bx` blocked.
- Rungs 4-8: acceptance text only, no gate yet.

Blocked is never a pass; keep the full North Star scope. Raw run evidence lives in `.tmp/`, compact evidence is
committed under each rung's `evidence/`. Orchestrator state: bennett/HANDOFF.md.
