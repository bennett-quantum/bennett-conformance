# bennett-conformance

## What this repo is

the demo ladder as executable gates (rungs 1-8), the certificate checker, the legality corpus runner, and the differential harness against native execution with instrumented classifiers (UBSan/ASan/MSan). A consumer of everything; nothing depends on it.

Part of the Bennett reboot: many small repositories, two published contracts
(`bennett-dialect-spec`, `bennett-isa-spec`). Depends on: everything, as a consumer only.

## Where the spec is

The specs live in `../bennett-isa-spec/spec/Bennett-ISA-Spec.md` and
`../bennett-dialect-spec/spec/Bennett-Dialect-Spec.md`, at the versions in
`SPEC_VERSIONS.toml`. The spec wins over any code, including code in this repo.

## Layout

- `clauses/`: clause dependency graph of both specs; IDs like `ISA-5.2-P3`; tool `clauses/clausegraph.py`.
- `suite/`: implementation-neutral conformance cases; runner `suite/run.py`.
- `proofs/`: Lamport-style structured proofs of the invariants the specs lean on; statements first, af ledgers later.
- `defects/`: defect-report conventions.
- `ladder/`: the demo ladder as executable gates.
- `checker/`: the certificate checker.
- `harness/`: the differential harness against native execution.

## Rules

The one rulebook is bennett/docs/process/COMMON.md; it binds here. Gate: `scripts/pre-push`. Tasks: `bd` (`bd prime` lists its commands).

Repo rule: every suite case carries clause tags (`clauses` in expect.json, IDs from clauses/); suite/run.py
   rejects a case without them. Defects are beads in the owning spec repo (bennett-isa-spec or bennett-dialect-spec) with labels `defect`, `isa`|`dialect`, and one of
   `open|ready|resolved|errata|editorial|not-a-defect`; a proposal for new semantics is not a defect.
