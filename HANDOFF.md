# Handoff: bennett-conformance
State (2026-10-31): clause graph (`clauses/`), neutral suite (`suite/`, 16 cases from
reference-interpreter@09fc712), defect conventions (`defects/`), proofs directory being
seeded with Lamport-style invariant statements (`proofs/`). Gate: `scripts/pre-push`
(install with `scripts/install-hooks.sh`); it needs an implementation binary
(`BENNETT_ISA_BINARY` or `../bennett-isa-spec-review` built).
Next: re-import suite cases when the interpreter's round-13 snapshot is accepted (the
importer refuses a dirty worktree); refine suite clause tags from section to paragraph IDs
using `clauses/clausegraph.py coverage`; install Graphviz for SVG; af (vibefeld) needs a Go
toolchain before proof ledgers replace the markdown proofs.
Specs pinned in SPEC_VERSIONS.toml. Orchestrator state: bennett/HANDOFF.md.
