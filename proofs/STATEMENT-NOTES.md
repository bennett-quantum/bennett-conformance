# Statement-view notes from the proof retag (ISA 0.8)

For the clause-graph owner. While retagging INV-1..INV-4 to statement ids we read every
statement of every cited clause in `clauses/ISA-0.8.statements.json`. The rows below
are statements that the view classifies `uncertain`, or classifies in a way the text
does not support. No proof works around them: the proofs cite the ids as they are,
and the kinds are left for the classifier to fix. Scope: statements of cited clauses
only, not a survey of the whole spec. Rules K1-K11 as in the statements JSON `rules`.

## Splitter

| Statement | Classifier says | Should be |
|---|---|---|
| `ISA-3.3-P2.S2` | one `definition` (K5): "**Regions are threaded the same way.** **Indexed operand** `w[i]` ... borrows wire i for one instruction." | two statements: S2 "Regions are threaded the same way." (`requirement`) and S3 "Indexed operand `w[i]` ..." (`definition`). The sentence end inside bold (`.**`) is not split. INV-1 1.3.7.2 cites S2 for the first sentence and INV-2 1.1.5 cites S2 for the second; after the fix INV-2 1.1.5 must move to S3. |

## K1: every fenced code block is `example`

These code blocks are normative tables or grammars, not examples. Several proofs rely
on them, so statement coverage currently counts normative content as `example`.

| Statement | Classifier says | Should be |
|---|---|---|
| `ISA-5.3-P4.S2` | `example` (K1) | `definition`: the Adjoint table itself (INV-1 throughout). |
| `ISA-4.4-P1.S1` | `example` (K1) | `definition`: the gate unitaries (INV-1, INV-2, INV-3). |
| `ISA-5.4-P1.S4` | `example` (K1) | `definition`: the fault-code numbering (INV-1 1.2.6.1). |
| `ISA-3.3-P1.S1` | `example` (K1) | `definition`: the type list, incl. "`reg[N]` a location ...; not linear" (INV-2 1.1.4). |
| `ISA-9.1-P1.S3` | `example` (K1) | `requirement`: the section schemas; "unknown sections or fields are errors" refers to it (INV-4). |
| `ISA-9.2-P1.S3` | `example` (K1) | `definition`: the envelope parts (1)-(6), incl. the (5b) sort rule (INV-4). |
| `ISA-16.2-P1.S1` | `example` (K1) | `requirement`: the object grammar (INV-4, not-covered note and gap 4). |

Suggested rule change: K1 should yield `example` only when the block is introduced as
an example (K6 context) and otherwise take the kind of the sentence that introduces it
(e.g. "**Adjoint table** (...):" is K5, so its table is `definition`).

## `uncertain` that should be decided

| Statement | Classifier says | Should be |
|---|---|---|
| `ISA-6.4-P1.S4` | `uncertain` (K3) | `requirement`: contains MUST; K3 propagated the *(non-normative)* marker of S3 to the next sentence. The marker should scope one sentence (or K4 should win). Cited by INV-3 1.6.3. |
| `ISA-4.7-P1.S1` | `uncertain` (K10) | `definition`: call forms and the equivalences `adj oracle_call ≡ oracle_call`, `ctl w <call> ≡` the call under `qpred w` (INV-1 1.1.1, 1.2.5, 1.3.9). |
| `ISA-5.3-P2.S1` | `uncertain` (K10) | `definition`: the instruction set admitted in an action body (INV-1 `Seg`, 1.3.10). |
| `ISA-5.1-P1.S4` | `uncertain` (K10) | `requirement`: "Predicate stack empty at entry and exit." (verbless but normative). |
| `ISA-5.3-P1.S2` | `uncertain` (K10) | `requirement`: "No fault outcome." |
| `ISA-3.3-P14.S1` | `uncertain` (K10) | `requirement`: how a `qreg[n]` argument fills a `wire[n]` / `region[A,W]` slot. |
| `ISA-4.4-P2.S1` | `uncertain` (K10) | `requirement`: gate forms, "all controlled on the active predicate". |
| `ISA-5.4-P1.S1` | `uncertain` (K10) | `definition`: Budget = (B, heap_limit, stack_limit, depth_limit). |
| `ISA-9.2-P2.S12` | `uncertain` (K10) | `definition`: `reference_program_hash` = the oracle's interface hash. |
| `ISA-9.2-P2.S13` | `uncertain` (K10) | `definition`: `params_hash`, `schedule_hash`. |
| `ISA-16.1-P1.S1` | `uncertain` (K10) | `requirement`: lexical rules (UTF-8, LF, single spaces, blank lines). |
| `ISA-16.1-P1.S2` | `uncertain` (K10) | `requirement` (or `definition`): the NAME/INT/FLOAT/STRING/HEX productions; INV-4 rests on NAME admitting `.`. |

Pattern: K10 fires on `X = ...` and `Term: list` fragments; an `=`/`:=` form or a
leading `**Term**:` should go to K5, and a verbless fragment inside a normative
paragraph should inherit the paragraph's kind rather than become `uncertain`.

## K5 overriding a requirement

A bold term makes K5 win even when the sentence's force is a rule, not a definition.

| Statement | Classifier says | Should be |
|---|---|---|
| `ISA-5.1-P1.S5` | `definition` (K5, bold "explicit cleanup instructions") | `requirement`: every `qalloc` matched by a `qfree` of a clean bundle, etc. (INV-2 1.5.3, INV-3 1.1.2, 1.7). |
| `ISA-5.1-P1.S6` | `definition` (K5, bold "`in` wires in actions") | `requirement`: where `in` wires may appear (INV-2 1.5.4). |
| `ISA-5.6-P2.S1` | `definition` (K5, bold "borrow frames") | `requirement` (it also defines borrow frames): borrowed wires are touched only inside borrow frames; tested by suite F (INV-1 1.3.5.1, INV-3 1.5.1). |
| `ISA-5.6-P3.S1` | `definition` (K5) | `requirement`, like its siblings (ii)-(iv) `ISA-5.6-P4.S1`..`ISA-5.6-P6.S1`. |
| `ISA-14.3-P3.S1` | `definition` (K5, bold "T3") | `requirement` (a normative theorem; there is no theorem kind) (INV-3 1.5.2). |
| `ISA-14.3-P5.S1` | `definition` (K5, bold "T5") | `requirement`, as for T3. |
| `ISA-9.2-P2.S4` | `definition` (K5) | same kind as `ISA-9.2-P2.S2`/`.S3` (`requirement`): closure set of actions. |
| `ISA-9.2-P2.S5` | `definition` (K5) | same: closure set of CC/R functions (INV-4 1.5.1). |
| `ISA-9.2-P2.S6` | `definition` (K5) | same: closure set of data symbols. |

## Not a classifier issue

Statement texts such as `ISA-9.2-P2.S7` (`` `globals. <name>` ``, `` `ptr_ width` ``),
`ISA-9.2-P2.S2` (`` `interface_ regions` ``), `ISA-5.6-P2.S1` ("permutation- only") and
`ISA-6.4-P1.S2` ("sched/ cert") contain a space where the spec source wraps a line
inside a code span or after a hyphen. CommonMark renders the same space, so this is
faithful to the source; it is editorial in `bennett-isa-spec`, not in the splitter.
