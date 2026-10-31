# Statement-view notes from the proof retag (ISA 0.8)

For the clause-graph owner. While retagging INV-1..INV-4 to statement ids we read every
statement of every cited clause in `clauses/ISA-0.8.statements.json` and recorded 29
statements that the view classified `uncertain` or in a way the text does not support.
Scope: statements of cited clauses only, not a survey of the whole spec.

The 2026-10-31 classifier fixes (design doc, "Statement layer": caption verbs, K3 scoped to
its own sentence, K5 narrowed with K13, normative fences K12, K14-K16) closed 24 of the 29
rows. The bold-end split moved one cited id: INV-2 1.1.5 now cites `ISA-3.3-P2.S3` ("Indexed
operand ..."); INV-1 1.3.7.2 keeps `ISA-3.3-P2.S2` ("Regions are threaded the same way.").
Only the rows below are still open. Rules K1-K16 as in the statements JSON `rules`.

## Still open

| Statement | Classifier says | Asked for | Why it is still open |
|---|---|---|---|
| `ISA-16.1-P1.S1` | `uncertain` (K10) | `requirement`: lexical rules (UTF-8, LF, single spaces, blank lines) | A verbless list of constraints that opens its paragraph: no normative neighbour for K16, no colon label for K15, no equation for K14. Nothing in the text separates it from a descriptive fragment; deciding it needs either a verb in the spec sentence or a rule keyed on the section (§16.1 *Lexical*), which would be a guess. Left `uncertain` on purpose. |
| `ISA-3.3-P1.S1` | `requirement` (K12) | `definition`: the type list, incl. "`reg[N]` a location ...; not linear" | Fixed from `example` to normative. The fence opens its clause straight after the §3.3 heading, so there is no introducing statement and K12 yields `requirement`; recognising a "pure type list" from the code block's content is not something the rules can do reliably. |
| `ISA-4.4-P1.S1` | `requirement` (K12) | `definition`: the gate unitaries | As above: normative now, but the fence follows the §4.4 heading directly, so its kind defaults to `requirement`. |
| `ISA-5.4-P1.S4` | `requirement` (K12) | `definition`: the fault-code numbering | Normative now; its introducing statement ("B is enforced by running exactly B steps; ...") is a requirement, so K12 gives `requirement`. |
| `ISA-5.3-P2.S1` | `requirement` (K15) | `definition`: the instruction set admitted in an action body | No longer `uncertain`. `**Body**` is not a defined term of the clause (no alias), so K5 cannot fire; K15 treats colon-labelled stems as requirements. Both kinds are defensible (it constrains action bodies). |
| `ISA-12.2-P1.S1` | `uncertain` | `definition`: the domain schema (`bundles`, `borrowed`, `regions`, inclusive bounds, enumeration order) | Met in INV-6 (2026-10-31). A verbless schema line that opens its clause, like `ISA-16.1-P1.S1`; its neighbour `ISA-12.2-P1.S2` is a requirement. Cited as is. |

## Not a classifier issue

Statement texts such as `ISA-9.2-P2.S7` (`` `globals. <name>` ``, `` `ptr_ width` ``),
`ISA-9.2-P2.S2` (`` `interface_ regions` ``), `ISA-5.6-P2.S1` ("permutation- only") and
`ISA-6.4-P1.S2` ("sched/ cert") contain a space where the spec source wraps a line
inside a code span or after a hyphen. CommonMark renders the same space, so this is
faithful to the source; it is editorial in `bennett-isa-spec`, not in the splitter.
