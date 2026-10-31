# Proofs

This directory holds proofs of the invariants the published specs lean on.
Each invariant is stated as a Lamport-style structured proof: a hierarchy of
numbered steps (`<1>1`, `<1>2`, `<2>1`, ...), where each step cites the clause
IDs it uses from `clauses/`. Statements come first; the proofs follow later.

Proofs are checked adversarially: a prover writes the steps and a verifier tries
to refute each one, using the `af` tool from
https://github.com/tobiasosborne/vibefeld once it is installed.

Every proof names the clause IDs it witnesses (CLAUDE.md rule 12).

Status: nothing is here yet. The directory is being set up; no proofs have been
written.
