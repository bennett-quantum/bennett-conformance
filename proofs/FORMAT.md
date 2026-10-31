# Proof format

Lamport-style hierarchical proofs ("How to Write a 21st Century Proof"); one step, one `af` node.

## Steps

```
<1>1. <claim>                         a level-1 step
      BY ISA-5.3-P4, ISA-4.5-P5       its leaf justification
<1>2. <claim>
      PROOF:                          a non-leaf step: its proof is the level below
  <2>1. <claim>
        BY <1>1, ISA-3.3-P3
  <2>2. QED
        BY <2>1
<1>3. QED
      BY <1>1, <1>2
```

- `<n>k` is step k at level n. Every level ends in a `QED` step that derives the
  enclosing claim from its siblings. Names are scoped as in Lamport: inside
  `<1>2`, `<2>1` means `<1>2`'s first child. Outside its scope a step is cited by
  its dotted path (`1.2.1`), which is also its af node id.
- Every step is a claim, a definition (`DEFINE ...`, no justification) or `ASSUME
  ... PROVE ...` (introduces hypotheses for its children). Every non-QED claim
  has exactly one justification: one leaf line, or `PROOF:` and children.

## Leaf kinds

| Leaf | Meaning |
|---|---|
| `BY <clause ids>` | the claim follows from the quoted clauses. |
| `BY <steps>` / `BY <steps>, <clause ids>` | follows from earlier steps (and clauses). |
| `ASSUMED: <why>` | the spec is silent; we adopt the stated reading. Every ASSUMED leaf is listed under "Spec gaps" with the sentence the spec would need. |
| `OPEN: <what>` | not proved; either the spec lacks what is needed or a counterexample to the literal text is known (then the leaf says `OPEN (refuted)` and gives it). Also listed under "Spec gaps". |

A `BY` leaf may end in `; algebra`: the rest is pure mathematics (linear, Boolean,
set algebra) over objects the cited clauses define; it never imports spec content.

## Citation rule

Cite only clause IDs from `clauses/ISA-0.8.json` (`ISA-5.6-P3`), or a section id
(`ISA-6.2`) when the whole section is meant. No page numbers, no section-sign
references, no code. Quote the clause in the step when its exact wording carries
the argument. The interpreter and the council reports are evidence (Witnesses),
never justification.

## File layout

Statement (informal, precise); Hypotheses; Proof; Witnesses; Spec gaps; Council
blockers as instances. `proofs/tags.json` maps `INV-n` to its cited clause ids.

## Mapping to af

One step = one af node, id = dotted path; the claim is the node statement; `BY`
clause ids become the node's external references; `BY` steps become dependency
edges; `ASSUMED` becomes an assumption node; `OPEN` a node left unvalidated for a
prover. A `PROOF:` step's children are its subgoals; `QED` is the closing node
(exact af command vocabulary to be fixed at migration time).
