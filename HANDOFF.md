# Handoff: bennett-conformance
State (2026-10-31 ~15:00Z): clause graph + statement layer (`clauses/`, `*.statements.{json,md}`
for ISA-0.8, ISA-0.9-draft, DLT-0.2.10; 107 tests), neutral suite (`suite/`, 16 cases pinned at
reference-interpreter@09fc712, still green against be9e6d1), four structured proofs (`proofs/`,
INV-1..4; 4 refuted 0.8 steps filed as beads in bennett-isa-spec). Gate: `scripts/pre-push`
(cargo + clause tests + suite tests + neutral suite against `BENNETT_ISA_BINARY` or the review
worktree build). Council rounds pass `proofs/INDEX.md` + `INV-*.md` as `--context`.
Next: retag proof citations to statement IDs (in progress); re-import suite cases once the
interpreter is accepted; rebuild graphs when the 0.9 draft lands its editorial pass (declared
`[deps:]` markers); Graphviz for SVG; Go toolchain before af ledgers replace markdown proofs.
Orchestrator state: bennett/HANDOFF.md.
