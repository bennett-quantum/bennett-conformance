# bennett-conformance

The demo ladder as executable gates, plus the tooling to judge any
implementation of the specs. A **consumer only**: nothing depends on it, and it
depends on everything through published contracts and binary edges.

- `ladder/rung-1` ... `ladder/rung-8`: acceptance tests from North Star section 8. Rungs 1-3 have gates (`gate.sh`); rungs 4-8 are acceptance text only so far. Rung 2 is complete; rungs 1 and 3 are blocked on upstream prerequisites. Authoritative status and evidence: each rung's `RESULTS.md` and `blockers.json`.
- `checker/`: Rust crate `bennett-cert-checker`, certificate checker (ISA spec section 12).
- `harness/`: differential harness against native execution, with instrumented classifiers (UBSan / ASan / MSan) deciding which inputs are in the defined domain (see `harness/README.md`).
- `clauses/`: clause dependency graph and statement layer of both specs ([README](clauses/README.md)).
- `suite/`: implementation-neutral conformance cases, run against any ISA implementation binary.
- `proofs/`: structured proofs of the invariants the specs lean on (INV-1..INV-6; see `proofs/INDEX.md`).
- legality corpus runner (dialect spec corpus): not written yet.

Gate: `scripts/pre-push` (clean worktree of the pushed commit). Current state and next steps: [HANDOFF.md](HANDOFF.md).
