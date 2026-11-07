# bennett-conformance

The demo ladder as executable gates, plus the tooling to judge any
implementation of the specs. A **consumer only**: nothing depends on it, and it
depends on everything through published contracts and binary edges.

- `ladder/rung-1` ... `ladder/rung-8`: acceptance gates and requirements from North Star section 8. Authoritative status and evidence: each rung’s `RESULTS.md` and `blockers.json`.
- `checker/`: Rust crate `bennett-cert-checker`, certificate checker (ISA spec section 12).
- `harness/`: black-box executor differential harness. Native-versus-compiled comparison with instrumented classifiers remains future work (see `harness/README.md`).
- `clauses/`: clause dependency graph and statement layer of both specs ([README](clauses/README.md)).
- `suite/`: implementation-neutral conformance cases, run against any ISA implementation binary.
- `proofs/`: structured proofs of the invariants the specs lean on (INV-1..INV-6; see `proofs/INDEX.md`).
- legality corpus runner (dialect spec corpus): not written yet.

Gate: `scripts/pre-push` (clean worktree of the pushed commit). Current state and next steps: [bennett/HANDOFF.md](../bennett/HANDOFF.md).
