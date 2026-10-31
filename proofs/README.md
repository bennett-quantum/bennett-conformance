# Proofs

Structured proofs of the invariants the Bennett specs lean on: reversibility of
matched compute/uncompute pairs, no clone, soundness of the static uncomputation
discipline, injectivity of the identity envelope. They are about the published
spec (ISA 0.8 for now), not about any implementation. Their main product is a
precise list of where the spec is silent or wrong: every `ASSUMED` and `OPEN`
leaf is a candidate defect or 0.9 proposal, with the sentence the spec would need.

## How to read

Start at [`INDEX.md`](INDEX.md) (status, leaf counts, refuted steps). Each
`INV-n-*.md` has an informal statement, a precise statement, its hypotheses as
clause ids, the proof, witnesses (suite cases and reference-interpreter tests),
council blockers that were instances, and spec gaps. Step syntax and leaf kinds
are in [`FORMAT.md`](FORMAT.md); clause ids resolve with
`python3 -I clauses/clausegraph.py impact clauses/ISA-0.8.json <id>`.

## How to add a theorem

1. Pick the next `INV-n`; write the statement first, using the spec's own terms.
2. Use `clausegraph.py impact` to find what the claim depends on; cite only clause
   ids from `clauses/` (FORMAT.md, citation rule).
3. Prove to at least depth 2. Where the spec is silent write `ASSUMED: <why>`;
   where it is missing or false write `OPEN: <what>` (with the counterexample if
   refuted). Do not invent spec text.
4. List every ASSUMED/OPEN under "Spec gaps" with the exact sentence needed.
5. Add `"INV-n": [cited ids]` to [`tags.json`](tags.json), a row to `INDEX.md`, and
   rerun `clausegraph.py coverage clauses/ISA-0.8.json --tags proofs/tags.json`
   (headless env, TMPDIR in the repo) to refresh the coverage line.

Proofs migrate later into `af` ledgers (https://github.com/tobiasosborne/vibefeld):
one step, one node; a verifier then tries to refute each step.
