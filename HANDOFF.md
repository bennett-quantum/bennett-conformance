# Handoff: bennett-conformance
State (2026-10-31): clause graph + statement layer (`clauses/`; ISA-0.8, ISA-0.9-draft and DLT statement
views, dialect and .dot outputs generated on demand; 113 tests); neutral suite (`suite/`, 16 cases re-imported
from the accepted reference-interpreter snapshot ad84e75, green); six structured proofs (`proofs/`, INV-1..6;
statement-level strong coverage 169/541 on ISA 0.8). Gate: `scripts/pre-push` (cargo + clause tests + suite
tests + neutral suite against `BENNETT_ISA_BINARY` or the review worktree build).
Next: proofs are consumed by council rounds as `--context`; a new proof is written when a council finding fits
no stated theorem. Graphviz for SVG and a Go toolchain for af ledgers remain optional.
Orchestrator state: bennett/HANDOFF.md.
