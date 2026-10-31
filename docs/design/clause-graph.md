# Clause dependency graph for the Bennett specs

Status: implemented (`clauses/clausegraph.py`, tests in `clauses/tests/`), 2026-10-31;
statement layer ("Statement layer" below) added the same day.
Usage and tagging conventions: [`clauses/README.md`](../../clauses/README.md).

## Purpose

Give every normative sentence of the published specs a stable handle and a place in a
dependency graph, so that we can

- ask what depends on a clause (`impact`), before changing it or when a council round
  changes it;
- ask whether two changes are independent (`independent`): neither clause lies in the
  other's cone and no clause is affected by both;
- follow the ISA from 0.8 to 0.9 (and the dialect onward) without inserting anchors into
  accepted spec bytes (`diff`);
- measure which clauses no test or proof witnesses (`coverage`).

The tool reads the specs; it never edits them.

## Inputs and outputs

Inputs: one spec markdown file per build; optionally (`--xspec`) the graph JSON of the
spec it cites. Today: ISA 0.8 (accepted), the ISA 0.9 working draft, and dialect 0.2.10
built against ISA 0.8.

`build` writes `clauses/<SPEC>-<version>.json` and `clauses/<SPEC>-<version>.dot`:

| JSON key | Content |
|---|---|
| `schema` | `bennett-clausegraph/1` |
| `spec`, `version` | e.g. `DLT`, `0.2.10` |
| `source` | file basename, sha256 of the bytes read, line count |
| `xspec` | spec, version and source sha256 of the `--xspec` graph, or null |
| `nodes` | root, sections, clauses (below) |
| `edges` | `{src, dst, kind, line, count}`; `contains` edges carry only src/dst/kind |
| `aliases` | alias → defining clause id; `alias_kinds`: `label` or `term` |
| `ambiguous_terms` | bold terms excluded from aliasing, with every location |
| `unresolved` | `{src, line, ref, reason[, fallback]}` for every reference not resolved to a node |
| `stats` | node, edge, alias and unresolved counts |

Output is deterministic (same input bytes, same JSON bytes); the test suite checks it.
The DOT file nests one `cluster_` subgraph per section, puts a folder-shaped node in each
cluster for references to the section itself, draws `refs` solid, `uses` dashed grey and
`xspec` blue, and omits `contains` (the clusters show it). `build --svg` runs
`dot -Tsvg` into `clauses/` when Graphviz is installed; it is not installed on the
build machine, so no SVG is committed. No viewer is ever invoked.

## Nodes and identifiers

- **Root** `ISA` / `DLT`.
- **Section** `<SPEC>-<n>` for `## n. Title`, `<SPEC>-<n.m>` for `### n.m Title`,
  `<SPEC>-<X>` for `## Appendix X. Title`. Text before the first `##` (status, versioning)
  is section `<SPEC>-front`; it is not `0`, because ISA 0.9 has a real `## 0. Changes from
  0.8`.
- **Clause** `<SPEC>-<section>-P<k>`: k-th clause directly in that section, in document
  order. Clause kinds: `para` (paragraph), `item` (list item, any nesting level, with its
  indented continuation paragraphs), `row` (table row), `code` (a fenced block with no
  earlier clause in its section). A fenced code block always belongs to the clause before
  it in the same section, including after a blank line; a run of code blocks with no
  prose between them therefore forms one clause.
- **Tables**: each body row is a clause; the header row is stored on every row as
  `table_header` (context, not hashed). In a *defining table* (below), consecutive rows
  with the same label merge into one clause (the dialect's N10 spans six rows).
- **Hash**: first 12 hex digits of sha256 over the clause text (code included) with
  whitespace runs collapsed to one space.
- **Markers**: `item_marker` is a list item's own number or leading `(x)` / `**(x)`;
  `markers` adds every inline enumerator `(1)`, `(5b)`, `(iii)`, `(a)` that follows
  whitespace or punctuation and precedes whitespace or punctuation (so ISA §9.2's
  envelope parts `(1)`…`(6)` and §5.2's construction steps `(1)`…`(4)` are findable).

Malformed input is a hard error (exit 2) naming the file and line: an unterminated code
fence, a heading other than `# Title`, `## n. Title`, `## Appendix X. Title` or
`### n.m Title` (so `####` and unnumbered `##` are rejected), a `### n.m` whose `n`
differs from the enclosing `## n.`, a duplicate section number, a second `#` title, a
duplicate label. Headings are recognised only outside code fences (the dialect's C
examples contain `#include`).

## What the parser recognises

### Labels (rule ids)

A label matches `[A-Z][A-Z0-9]*([-.][A-Z0-9]+)*`: `PT3`, `N21`, `TDL-5`, `BD-E-RESIDUAL`,
`BLK-1`, `E16.9`, `DB`. Three definition forms, read from the actual specs:

1. **Defining table row.** A table whose first header cell is one of `Id`, `Rule`,
   `Step`, `Code`, `Check`, `Suite`, `#`, and every body row of which starts with a label
   cell: `PT3`, `` `BD-E-RESIDUAL` ``, `**BLK-1** = ISA-4`, or a comma list `DR, DF`. Each
   such row defines its labels. The all-rows condition is what keeps dialect Appendix C
   (header `Rule`, but cells like `N0: the coroutine passes` and a bare `N11` that
   *refers* to N11) from redefining rules. Single-letter labels (ISA suites `A`–`H`, the
   0.9 change entry `V`) are not aliases: they would match the article "A" and every
   variable named `C`.
2. **Bold lead.** A clause (or list item) whose text starts `**LABEL …**`: dialect
   `**TDL-5 Stores to outliving objects (the store rule).**`, `**P-1.**`, ISA
   `1. **C1 capture**:`, `**T3** (permutation theorem).`, 0.9 `**BLOCK-IMAGE**`.
3. **List lead.** A list item starting `LABEL:` or `` `LABEL`: ``: dialect `- MC1: …`.

Forms 2 and 3 need a digit or hyphen in the label (so `**RAM**`, `- T:` are not labels).

**Uniqueness.** A label defined by two defining-table rows that are not adjacent is an
error naming both lines. When a table defines a label, bold-lead and list-lead
occurrences are elaborations, not definitions (`**PT3 geometry-changing bitcast
witnesses.**`, `**V2.**` examples, the `` - `BD-E-HANDLE-ESCAPE`: `` item in §6.4): they get
a `uses` edge to the table row like any other mention. Without a table definition the
first bold or list lead defines the label; a later lead with the *same* bold text is an
error, a later lead with different text (`**BLOCK-IMAGE witness**`) is an elaboration.
No real spec trips the duplicate error today.

### Defined terms (bold)

A bold span `**…**` in prose (not in code) defines a term when, after stripping a
trailing `.:,;`, it has at most 4 words and 48 characters, starts with a letter or
backtick, contains none of `§()=|"[]{}<>*`, has balanced backticks, is at least two
characters, and its first word is not a function word (`every`, `or`, `no`, `only`,
`default`, `MUST`, …). Three further rules, each forced by the real text:

- **Captions are not terms.** A bold span that opens its clause and ends with `.` inside
  the bold (`**Requirement.**`, `**Bounds.**`, `**Control.**`) is a paragraph caption,
  except in a Terminology section, where `**Tier.**` is a definition.
- **Single plain words only in Terminology.** Outside a section titled `Terminology`, a
  one-word term made only of letters (`**constant**`, `**result**`, `**Body**`,
  `**involution**`) is dropped unless it is an acronym (`CC`, `SCCs`). Measured on the
  dialect before this rule, the clauses bolding `constant`, `row` and `C` drew 72, 62
  and 71 `uses` edges, nearly all from ordinary English. Hyphenated or code-like single words stay
  (`self-disjoint`, `selection_identity`).
- **Ambiguity.** A term bolded in two or more clauses is an alias only if exactly one of
  those clauses is in a Terminology section (ISA `lproc`, dialect `variant`, `trapped`);
  otherwise it is excluded and recorded in `ambiguous_terms` with every location, and
  `build` prints a warning. Dialect 0.2.10 has two: `full version string` (front matter
  and §23) and `oracle region` (§7.1 and §7.3). ISA 0.8 and the 0.9 draft have none.

A label always wins over a term of the same text.

### Section references

| Form | Example | Resolves to |
|---|---|---|
| `§n`, `§n.m` | `§5.6`, `§12` | the section node |
| `§§a, b` (also `and`, `/`) | `§§4.5, 4.7`, `§§17.1/17.3` | each section |
| range | `§16.2–§16.4`, `§3–§7`, `§4.5–4.7` | every section of the endpoints' depth between them, in document order |
| item | `§9.2 (5b)`, `§7.5(a)`, `§7.5b`, `§17 item 7`, `§6.5 rule 1`, `§6.2 items 2–4`, `§6.5 rules 1 and 2` | the clause in that section (or its subsections) whose `item_marker` matches, else the first whose inline `markers` match; if none, the section, recorded as `item-not-found` with a `fallback` |
| possessive | `§5.2's` | the section |

The word before `§` (across line breaks; `(` and whitespace skipped; possessive `'s`
stripped) sets the target:

- `ISA`, `` `bennett-isa-spec` `` → the ISA (from the dialect: an `xspec` edge, resolved
  against the `--xspec` graph; without one, `unresolved: xspec-not-loaded`). `dialect`,
  `` `bennett-dialect-spec` `` → the dialect. Inside the spec of the same name these are local.
- `NS`, `north star` / `star`, `` `Bennett-NorthStar.md` ``, `SQLite`, `findings` (spike
  findings), `outline`, `DL` (0.9's abbreviation for the dialect outline) → external,
  recorded as `external:<name>`.
- A version number: `ISA 0.8 §x` → the ISA; `dialect draft 0.2.1 §x` → the dialect; a
  bare version other than the spec's own (`(0.8 §7.3)` in ISA 0.9, `0.1 §1.4` in the
  dialect) → `external:<SPEC>-<version>`, a prior version.
- Anything else, including a preceding `§x.y` (`per §3.3 (§3.1: …)`), is local.

A prefix carries across a list joined only by `,`, `/`, `–`, `and`, `or`: in
`(ISA §4.7, §3.1)` both are ISA; `(ISA §4.1); §7.4` is ISA then local.

### Uses (alias mentions)

Every clause is scanned for every alias of its spec, excluding mentions in the defining
clause itself.

- Labels: case-sensitive, in prose and code, delimited so that `N2` does not match
  `N21`, `xN2`, `N2-x` or `E17.2`, and `TDL-5` does not match `TDL-50` (it does match
  `TDL-5(b)`). An en-dash range of one family (`PT3–PT10`, `C1–C5`) adds a `uses` edge to
  every defined label in between.
- Terms: case-sensitive, prose only (code is not scanned: `commit` in a code listing is
  not the commit step), word-delimited, with an optional `s`/`es` plural (`borrow frame`
  does not match, `lprocs` does).

## Edges

| Kind | From → to | Source |
|---|---|---|
| `contains` | root → section → subsection → clause | headings and positions |
| `refs` | clause → section or clause, same spec | `§` references |
| `xspec` | dialect clause → ISA section or clause | `ISA §…` with `--xspec` |
| `uses` | clause → defining clause | label and term mentions |

Edges are deduplicated per (src, dst, kind) with a `count` and the first line.

## Dependency semantics

"Y depends on X" means Y cites X: a `refs`/`xspec`/`uses` edge from Y to X, or a `refs`/
`xspec` edge from Y to a section containing X (citing `§5.6` cites every clause in it).
The forward cone of A is what A transitively depends on; the reverse cone is what
transitively depends on A; both include A (and, for a section, its clauses).

Two closures are implemented:

- **`full`**: the transitive closure of that relation. Every clause reached is lifted
  to its enclosing sections, so anything citing *those* sections is reached next.
- **`anchored`** (default): section citations are followed one hop, at the query end
  only. The reverse cone of X starts from X and its enclosing sections (so `§5.6` and
  `§5` citations count), then chains through name-level edges only: label and term
  mentions, item references, references to the query's sections. Symmetrically, the
  forward cone expands the query clause's own section citations and then follows only
  edges to clauses.

The full closure is sound but useless on these specs, because they are densely
cross-referenced and the terminology clauses cite whole sections (dialect §2: "Abstract
object, slot, value set: §5"; "Tier … L (lifted …, ISA §3.7)"). Measured on the
committed graphs (300 random clause pairs per spec, seed 7; ISA 0.9 at draft revision
329ac8a8dcc6, before its rebuild for the statement layer):

| Graph | mean reverse cone, full | mean reverse cone, anchored | independent pairs, full | independent pairs, anchored |
|---|---|---|---|---|
| ISA-0.8 | 104 | 13 | 5 / 300 | 87 / 300 |
| ISA-0.9-draft | 273 | 34 | 0 / 300 | 43 / 300 |
| DLT-0.2.10 | 783 | 70 | 1 / 300 | 36 / 300 |

Under `full`, 1123 of 1243 dialect clauses have 800 or more dependents and the rest have
none: the reverse closure is one giant component. `anchored` keeps the depth structure
meaningful (ISA §5.6: 17 dependents across both specs instead of 1006). Use `full` when a
conservative bound is wanted, and `--max-depth` to cut either closure.

`independent a b` follows the brief: it succeeds only if the forward cones are disjoint
*and* the reverse cones are disjoint (section nodes excluded from the comparison;
clauses only). Two clauses that both use a common Terminology entry share that entry in
their forward cones and are reported dependent; the shared nodes are printed so the
reason is visible. That is deliberately strict; most dialect pairs fail it (table above).

`impact` lists dependent clauses by BFS depth. `top` ranks clauses by number of
transitive dependents (ties: direct dependents, i.e. depth 1).

## Stability across versions: `diff`

`diff a.json b.json` matches the clauses of two graphs of the same spec in tiers:

1. **hash**: identical normalised text; repeated hashes pair in document order;
2. **alias**: same (section number, alias), then the same alias in any section (labels
   are unique per spec, so a moved PT3 is still PT3);
3. **similar** (extension): remaining clauses of the same section number, paired greedily
   by `difflib` ratio ≥ 0.6. Without it every edited paragraph without an alias would
   count as removed plus added.

Each matched pair is *unchanged* (same id, same hash), *moved* (different id: renumbered
within a section or moved to another), *changed* (different hash) or both. Unmatched
clauses are *added* / *removed*. `--list` prints every pair; `--json` everything.
Tags and external references should use aliases where they exist, and be re-targeted
through `diff --list` where they do not.

### ISA 0.8 → 0.9-draft

Built from `Bennett-ISA-Spec.md` (sha256 90e61d9c377c…) and `Bennett-ISA-Spec-0.9-draft.md`
(sha256 e98efc52c9be…, rebuilt 2026-10-31 for the statement layer; the draft is being
edited concurrently, so these numbers describe that revision only):

```
diff ISA-0.8 -> ISA-0.9-draft: 314 -> 556 clauses; matched 289 {"alias": 30, "hash": 209, "similar": 50}; unchanged 162; moved 79 (renumbered only 47); changed 80 (in place 48); added 267; removed 25
```

| § | unchanged | moved | changed | moved+changed | added | removed |
|---|---|---|---|---|---|---|
| 0 | 0 | 0 | 0 | 0 | 95 | 0 |
| 1 | 5 | 0 | 1 | 0 | 1 | 1 |
| 2 | 7 | 1 | 1 | 0 | 1 | 0 |
| 3 | 12 | 29 | 6 | 16 | 22 | 2 |
| 4 | 56 | 6 | 12 | 2 | 25 | 4 |
| 5 | 14 | 0 | 4 | 0 | 6 | 3 |
| 6 | 17 | 0 | 1 | 0 | 1 | 1 |
| 7 | 17 | 1 | 2 | 4 | 7 | 2 |
| 8 | 5 | 8 | 2 | 0 | 6 | 0 |
| 9 | 3 | 0 | 2 | 2 | 7 | 1 |
| 10 | 1 | 0 | 6 | 0 | 8 | 0 |
| 11 | 2 | 1 | 1 | 0 | 43 | 0 |
| 12 | 3 | 1 | 2 | 3 | 12 | 2 |
| 13 | 0 | 0 | 1 | 0 | 0 | 0 |
| 14 | 6 | 0 | 2 | 0 | 1 | 0 |
| 15 | 1 | 0 | 1 | 0 | 6 | 6 |
| 16 | 0 | 0 | 1 | 5 | 21 | 2 |
| 17 | 13 | 0 | 1 | 0 | 5 | 1 |
| front | 0 | 0 | 2 | 0 | 0 | 0 |
| total | 162 | 47 | 48 | 32 | 267 | 25 |

Reading: §0 (95 clauses) is the new change log; §11 gains §11.1 (reference-library
contract; §11 as a whole gains 43 clauses), §10 gains §10.1 (launch estimate); §3 is
mostly renumbering (29 hash-identical clauses moved because paragraphs were inserted
earlier in §3.2, §3.3, §3.6 and §3.7). The 6 removed and 6 added clauses of §15 are the
suite table rows A, B, C, F, G, H, each rewritten beyond the similarity threshold.

## Results on the committed graphs

| Graph | nodes | sections | clauses | edges | contains | refs | uses | xspec | aliases (labels + terms) | unresolved |
|---|---|---|---|---|---|---|---|---|---|---|
| ISA-0.8 | 384 | 69 | 314 | 649 | 383 | 161 | 105 | 0 | 116 (16 + 100) | 3 |
| ISA-0.9-draft | 633 | 76 | 556 | 1609 | 632 | 619 | 358 | 0 | 241 (98 + 143) | 14 |
| DLT-0.2.10 | 1413 | 169 | 1243 | 4092 | 1412 | 1071 | 1404 | 205 | 337 (179 + 158) | 129 |

Unresolved references by reason (all are references outside the spec being parsed;
none is a parse failure):

| Graph | reasons |
|---|---|
| ISA-0.8 | `external:north-star` 3 |
| ISA-0.9-draft | `external:ISA-0.8` 6, `external:NS` 1, `external:dialect-outline` 1, `external:north-star` 4, `external:outline` 1, `xspec-not-loaded` 1 |
| DLT-0.2.10 | `external:DLT-0.1` 2, `external:NS` 105, `external:SQLite` 7, `external:north-star` 7, `external:outline` 2, `external:spike-findings` 6 |

The dialect's 205 `xspec` edges are all resolved against ISA 0.8 (none
`no-such-section`); built without `--xspec`, the same 200 `ISA §` references are
listed as `xspec-not-loaded` (205 edges > 200 references because ranges and item lists
expand to several targets). ISA 0.9 has one dialect reference (`dialect draft 0.2.1 §25.1`),
unresolved because no dialect graph is loaded when the ISA is built.

### Example `impact` outputs (anchored)

ISA §5.6, borrowed systems and borrow frames, with the dialect loaded (`impact
clauses/ISA-0.8.json ISA-5.6 --with clauses/DLT-0.2.10.json`):

```
impact of ISA-5.6 (§5.6 Borrowed systems and borrow frames) (closure anchored): 17 dependent clauses
depth 1 (10): DLT-1.3-P1 [efficiency item], DLT-18.3-P3, ISA-1.2-P1, ISA-1.2-P2, ISA-4.3-P2, ISA-5.2-P2, ISA-5.3-P1 [Permutation action], ISA-14.2-P1, ISA-14.3-P6 [T6], ISA-15-P3
depth 2 (6): DLT-8.6-P6, DLT-11.4-P1, ISA-4.2-P29 [reference body], ISA-5.7-P1, ISA-11-P2 [expansion library, generated operations], ISA-12.3-P2
depth 3 (1): ISA-14.1-P1
```

Dialect PT3 (`impact clauses/DLT-0.2.10.json PT3 --with clauses/ISA-0.8.json`):

```
impact of DLT-5.2-P4 [PT3] (closure anchored): 155 dependent clauses
depth 1 (18): DLT-1.1-P4, DLT-1.2-P4, DLT-1.5-P3 [P-1], DLT-2-P10 [Abstract object, slot, value set], DLT-4.1-P3 [S2], DLT-5.2-P17, DLT-6.2-P1 [bitwise copies], DLT-7.2-P1, DLT-8.4-P1, DLT-11.4-P1, DLT-12.2-P5 [N3], DLT-14.2-P1, DLT-19.4-P1 [pointer-rich], DLT-21.3-P2 [DB], DLT-A-P14 [A13], DLT-B-P10, DLT-B-P46, DLT-front-P3
depth 2 (61): DLT-1.4-P2, DLT-3.2-P2, DLT-3.4-P20 [dispatch domain], DLT-3.4-P28, DLT-3.8-P3 [Region functions], DLT-4.1-P11, DLT-4.5-P3 [runtime shim], DLT-5.1-P12 [abstract value], DLT-5.2-P1, DLT-5.2-P6 [PT5], DLT-5.2-P7 [PT6], DLT-5.2-P9 [PT8], DLT-5.2-P10 [PT9], DLT-5.2-P13 [PT12], DLT-5.2-P16, DLT-5.3-P1, DLT-5.4-P5, DLT-5.4-P8, DLT-5.4-P9, DLT-6.1-P1 [register tag], DLT-6.2-P3, DLT-6.2-P6, DLT-6.3-P1, DLT-6.4-P1, DLT-6.4-P2, DLT-6.5-P2, DLT-6.5-P3, DLT-6.5-P11, DLT-6.5-P12, DLT-6.5-P13, DLT-6.9-P6, DLT-7.7-P3, DLT-8.7-P3, DLT-8.8-P8, DLT-9.5-P2, DLT-9.5-P6, DLT-10-P40, DLT-10-P42, DLT-12.2-P2 [N0], DLT-12.2-P27 [N25], DLT-12.2-P36 [N34, R-tier budget instrumentation points], DLT-12.2-P46, DLT-12.7-P5, DLT-12.8-P1, DLT-12.8-P14, DLT-12.8-P15, DLT-13.1-P24, DLT-13.1-P25, DLT-14.3-P4, DLT-14.3-P10, DLT-18.8-P27, DLT-20.7-P4, DLT-20.7-P5, DLT-21.4-P1, DLT-23-P2, DLT-25.1-P2 [BLK-1], DLT-25.1-P9, DLT-A-P16 [A15], DLT-A-P17 [A16], DLT-B-P19, DLT-B-P32
depth 3 (50): DLT-2-P18 [Blocker], DLT-3.4-P21, DLT-3.4-P23, DLT-4.1-P8 [S7], DLT-5-P1, DLT-5.2-P3 [PT2], DLT-5.2-P8 [PT7], DLT-5.2-P12 [PT11], DLT-5.2-P15, DLT-5.2-P20, DLT-5.2-P21, DLT-5.3-P4, DLT-5.4-P7, DLT-6.4-P3, DLT-6.5-P5, DLT-6.5-P6, DLT-6.5-P7, DLT-6.5-P10 [blocker point], DLT-6.5-P14, DLT-6.5-P15, DLT-6.9-P8, DLT-6.9-P10, DLT-7.6-P2, DLT-7.6-P5, DLT-8.2-P1, DLT-9.5-P3 [Launch-supplied], DLT-10-P39, DLT-12.2-P41, DLT-12.2-P47, DLT-12.4-P5 [V4], DLT-12.9-P1, DLT-16.7-P3, DLT-17.6-P1 [step counter], DLT-22.2-P1, DLT-24-P12 [D29], DLT-A-P4 [A3], DLT-B-P3, DLT-B-P8, DLT-B-P9, DLT-B-P31, DLT-B-P48, DLT-B-P51, DLT-B-P56, DLT-C-P2, DLT-C-P3, DLT-C-P4, DLT-C-P5, DLT-C-P6, DLT-C-P7, DLT-C-P17
depth 4 (13): DLT-5.2-P5 [PT4], DLT-5.2-P14, DLT-6.6-P1 [handle parameter], DLT-7.5-P3, DLT-12.4-P11 [V10], DLT-12.4-P22, DLT-17.5-P11, DLT-17.6-P4, DLT-18.7-P1, DLT-25.1-P8 [Blocker report], DLT-25.3-P12 [LLVM changes needed], DLT-B-P47, DLT-B-P53
depth 5 (8): DLT-6.9-P3, DLT-7.7-P10, DLT-12.4-P28, DLT-18.6-P1, DLT-19.2-P5, DLT-22.1-P16 [BD-E-REVERSIBLE-MEASURE], DLT-25.1-P5 [BLK-4], DLT-A-P5 [A4]
depth 6 (4): DLT-7.2-P6 [trap body], DLT-7.7-P11, DLT-8.5-P7 [Build-time snapshots], DLT-B-P12
depth 7 (1): DLT-7.7-P7
```

Dialect N21 (`impact clauses/DLT-0.2.10.json N21 --with clauses/ISA-0.8.json`):

```
impact of DLT-12.2-P23 [N21, vector scalarisation] (closure anchored): 206 dependent clauses
depth 1 (38): DLT-1.1-P11, DLT-1.5-P3 [P-1], DLT-1.5-P10, DLT-2-P17 [Materialised module, Reference object], DLT-3.1-P2, DLT-4.1-P8 [S7], DLT-5.2-P4 [PT3], DLT-5.2-P16, DLT-8.1-P5, DLT-8.2-P1, DLT-10-P16 [classical twin, slice function], DLT-12.1-P1 [materialised module], DLT-12.2-P2 [N0], DLT-12.2-P17 [N15], DLT-12.2-P34 [N32, floating point], DLT-12.2-P44, DLT-12.4-P5 [V4], DLT-12.4-P9 [V8], DLT-12.4-P26, DLT-12.5-P1, DLT-12.9-P1, DLT-12.9-P2, DLT-12.9-P3, DLT-13.2-P1, DLT-16.1-P1, DLT-16.2-P5, DLT-16.4-P14, DLT-18.1-P2, DLT-21.3-P5 [DN], DLT-A-P16 [A15], DLT-A-P17 [A16], DLT-B-P22, DLT-B-P23, DLT-B-P33, DLT-B-P46, DLT-B-P53, DLT-C-P1, DLT-C-P14
depth 2 (45): DLT-1.2-P4, DLT-1.5-P12, DLT-4.1-P11, DLT-5.2-P11 [PT10], DLT-5.2-P14, DLT-5.2-P17, DLT-6.2-P1 [bitwise copies], DLT-6.5-P9, DLT-10-P17 [slice invocation], DLT-12.2-P11 [N9], DLT-12.2-P19 [N17], DLT-12.2-P21 [N19], DLT-12.2-P38 [Completeness obligation], DLT-12.2-P41, DLT-12.2-P43, DLT-12.4-P10 [V9], DLT-12.4-P11 [V10], DLT-12.4-P19, DLT-12.4-P22, DLT-12.4-P27, DLT-12.4-P28, DLT-12.6-P5, DLT-12.6-P6 [left fold], DLT-13-P1, DLT-13.1-P24, DLT-13.6-P4, DLT-17.7-P1 [native comparison binary], DLT-18.1-P1, DLT-19.5-P1, DLT-21.1-P2 [L1], DLT-21.4-P1, DLT-A-P7 [A6], DLT-A-P24 [A23], DLT-B-P10, DLT-B-P27, DLT-B-P36, DLT-B-P47, DLT-C-P2, DLT-C-P3, DLT-C-P4, DLT-C-P5, DLT-C-P6, DLT-C-P7, DLT-C-P13, DLT-C-P28
depth 3 (48): DLT-1.2-P3, DLT-1.4-P2, DLT-5.2-P13 [PT12], DLT-5.2-P18, DLT-6.9-P10, DLT-7.5-P3, DLT-8.8-P2, DLT-10-P28, DLT-10-P30, DLT-10-P31, DLT-10-P36 [TDL-8, declared fault register], DLT-10-P37 [TDL-9], DLT-10-P39, DLT-12.2-P1, DLT-12.2-P48, DLT-12.2-P49, DLT-12.2-P50, DLT-12.9-P11, DLT-16.2-P3, DLT-16.2-P4, DLT-16.2-P15, DLT-16.4-P37, DLT-16.4-P41, DLT-16.4-P42, DLT-16.4-P43, DLT-16.4-P44, DLT-16.4-P45, DLT-16.4-P46, DLT-16.6-P1, DLT-16.6-P2, DLT-16.6-P3, DLT-16.7-P5, DLT-16.7-P7, DLT-16.8-P2, DLT-16.9-P2, DLT-16.10-P1, DLT-16.11-P1, DLT-16.11-P3, DLT-16.11-P4, DLT-17.1-P6 [Single thread], DLT-18.8-P20, DLT-18.8-P21, DLT-20.3-P1, DLT-20.7-P9, DLT-21.1-P6 [L5], DLT-24-P8 [D24], DLT-B-P28, DLT-B-P29
depth 4 (30): DLT-5.2-P15, DLT-6.5-P5, DLT-6.5-P6, DLT-6.5-P7, DLT-6.5-P14, DLT-7.1-P7 [action region], DLT-7.7-P10, DLT-9.1-P6, DLT-9.3-P2, DLT-9.4-P1, DLT-10-P35 [TDL-7], DLT-10-P47 [TDL-12], DLT-16.5-P4, DLT-17.5-P12, DLT-18.2-P5, DLT-18.2-P10, DLT-18.3-P14, DLT-18.3-P16, DLT-18.4-P1 [Leak rule], DLT-18.6-P1, DLT-18.8-P14, DLT-19.2-P5, DLT-22.1-P16 [BD-E-REVERSIBLE-MEASURE], DLT-25.1-P5 [BLK-4], DLT-A-P5 [A4], DLT-A-P10 [A9], DLT-B-P17, DLT-B-P48, DLT-B-P55, DLT-B-P56
depth 5 (24): DLT-1.5-P20, DLT-2-P13 [Arena, Coherence state, Coherent slice, Generation], DLT-2-P18 [Blocker], DLT-4.2-P2, DLT-7.2-P6 [trap body], DLT-7.4-P2, DLT-7.4-P4, DLT-7.5-P2 [deferred measurement], DLT-7.7-P11, DLT-8.5-P7 [Build-time snapshots], DLT-10-P1, DLT-10-P2 [TDL-1], DLT-10-P9 [MC2], DLT-10-P14 [TDL-3], DLT-10-P33, DLT-10-P41, DLT-11.1-P6, DLT-14.4-P10, DLT-18.2-P4, DLT-18.8-P24, DLT-22.2-P1, DLT-A-P11 [A10], DLT-A-P12 [A11], DLT-B-P12
depth 6 (15): DLT-4.2-P4, DLT-7.6-P1, DLT-7.7-P7, DLT-7.7-P9, DLT-10-P3 [TDL-2], DLT-10-P5, DLT-10-P44, DLT-18.2-P7, DLT-18.2-P8, DLT-18.3-P9, DLT-18.8-P3, DLT-22.1-P15 [BD-E-REGION-MEASURE], DLT-25.1-P9, DLT-25.2-P8 [Certificate fields], DLT-A-P21 [A20]
depth 7 (6): DLT-4.3-P1, DLT-5.4-P5, DLT-7.7-P8, DLT-10-P45, DLT-18.1-P6, DLT-A-P6 [A5]
```

The same three under `--closure full`:

```
impact of ISA-5.6 (§5.6 Borrowed systems and borrow frames) (closure full): 1006 dependent clauses
impact of DLT-5.2-P4 [PT3] (closure full): 866 dependent clauses
impact of DLT-12.2-P23 [N21, vector scalarisation] (closure full): 866 dependent clauses
```

### Clauses with the most dependents (anchored)

ISA 0.8, ISA dependents only:

| clause | transitive dependents | direct dependents | aliases |
|---|---|---|---|
| ISA-3.7-P8 | 39 | 14 | C2 |
| ISA-3.7-P9 | 38 | 13 | C3 |
| ISA-3.7-P10 | 38 | 13 | C4 |
| ISA-3.7-P11 | 38 | 13 | C5 |
| ISA-3.7-P14 | 38 | 13 | R1 |
| ISA-3.7-P16 | 38 | 13 | R3 |
| ISA-3.7-P17 | 38 | 13 | R4 |
| ISA-3.7-P18 | 38 | 13 | R5 |
| ISA-3.7-P3 | 38 | 11 |  |
| ISA-3.7-P4 | 38 | 11 | Initial allocator state |

ISA 0.8 counting dialect dependents (`--with clauses/DLT-0.2.10.json`): §3.7 is cited
by the dialect's definition of the L tier (`DLT-2-P7`), which 111 dialect clauses use,
so every §3.7 clause reaches about 300 clauses:

| clause | transitive dependents | direct dependents | aliases |
|---|---|---|---|
| ISA-3.7-P8 | 315 | 22 | C2 |
| ISA-3.7-P9 | 314 | 21 | C3 |
| ISA-3.7-P10 | 314 | 21 | C4 |
| ISA-3.7-P11 | 314 | 21 | C5 |
| ISA-3.7-P14 | 314 | 21 | R1 |
| ISA-3.7-P16 | 314 | 21 | R3 |
| ISA-3.7-P17 | 314 | 21 | R4 |
| ISA-3.7-P18 | 314 | 21 | R5 |
| ISA-3.7-P3 | 314 | 19 |  |
| ISA-3.7-P4 | 314 | 19 | Initial allocator state |

ISA 0.9-draft:

| clause | transitive dependents | direct dependents | aliases |
|---|---|---|---|
| ISA-3.7-P13 | 103 | 37 | C2 |
| ISA-3.7-P14 | 102 | 36 | C3 |
| ISA-3.7-P16 | 101 | 36 | C5 |
| ISA-3.7-P15 | 101 | 35 | C4 |
| ISA-3.7-P19 | 101 | 35 | R1 |
| ISA-3.7-P21 | 101 | 35 | R3 |
| ISA-3.7-P22 | 101 | 35 | R4 |
| ISA-3.7-P23 | 101 | 35 | R5 |
| ISA-3.7-P3 | 101 | 33 |  |
| ISA-3.7-P7 | 101 | 33 |  |

Dialect 0.2.10 (with ISA 0.8 loaded):

| clause | transitive dependents | direct dependents | aliases |
|---|---|---|---|
| DLT-4.3-P1 | 267 | 8 |  |
| DLT-2-P7 | 266 | 111 | CC, Tier, variant |
| DLT-6.1-P1 | 225 | 9 | register tag |
| DLT-2-P3 | 213 | 116 | Handle, Register tag, register, wire |
| DLT-12.2-P23 | 206 | 38 | N21, vector scalarisation |
| DLT-12.6-P1 | 205 | 16 |  |
| DLT-12.6-P2 | 205 | 16 |  |
| DLT-12.6-P3 | 205 | 16 |  |
| DLT-12.6-P4 | 205 | 16 |  |
| DLT-12.6-P7 | 205 | 16 |  |

## Statement layer

The graph's nodes are paragraphs, list items and table rows. Proofs, council findings and
tests want to cite a single normative sentence, and two edits of one paragraph may be
independent. `statements` derives a statement layer from a graph JSON (never from or into
spec bytes): every clause is split into atomic, Lamport-numbered statements with an id, a
kind, a hash and dependencies.

```sh
python3 -I clauses/clausegraph.py statements clauses/ISA-0.8.json          # writes clauses/ISA-0.8.statements.{json,md}
python3 -I clauses/clausegraph.py statements clauses/DLT-0.2.10.json       # --xspec defaults to the sibling ISA-0.8.json
```

### Ids, numbering, hash

- **Statement** `<clause-id>.S<k>`: k-th statement of the clause in text order
  (`ISA-5.1-P1.S3`); Lamport number `<1>k`.
- **Sub-statement** `<clause-id>.S<k>.<j>`: j-th item of an enumeration `(1)…(n)`,
  `(a)…`, `(i)…` inside statement k; Lamport number `<2>j`. The parent keeps the whole
  sentence (citing it cites every item); each item is also its own statement.
- **Hash**: first 12 hex digits of sha256 over the statement text with whitespace
  collapsed, the list marker (`- `, `3. `) and the outer table pipes removed, and any
  `[deps: …]` marker removed: adding a marker changes the clause hash but not the
  statement hash.
- Ids are positional like clause ids; `diff --statements` bridges versions (below).

JSON (`bennett-statements/1`): `spec`, `version`, `graph` (source file and sha256 of the
graph it was derived from; queries refuse a stale layer), `xspec`, `rules` (the
classifier rules below, verbatim), `statements`, `unresolved` (declared markers that did
not resolve) and `stats` (`by_kind`, `by_rule`, `by_section`). Each statement carries `id`,
`clause`, `section`, `n`, `level`, `parent`, `kind`, `rule`, `modal` (RFC keywords found),
`defines`, `text`, `hash`, `line` (first source line), `marker` (the raw `[deps:]` bodies
or null), `deps.declared` and `deps.inferred`, and `uncertain` (the reason) for that
kind. The `.md` file is a per-section Lamport listing with per-section kind counts.

### Splitting rules

1. **Blocks.** A clause's text splits into prose paragraphs (blank lines) and fenced code
   blocks; each code block is one statement. Paragraph breaks always end a statement.
2. **Sentence ends** are `.`, `!`, `?` (optionally followed by `*`, `_`, quotes) followed
   by whitespace and a sentence start: an upper-case letter, digit, `` ` ``, `§` or `[`,
   possibly behind `*`, `_`, `(` or a quote. A lower-case continuation is never a
   boundary.
3. **No split** inside inline code spans (backtick runs of equal length), inside matched
   `( )` and `[ ]`, at `..` / `...`, or after the abbreviations `e.g.`, `i.e.`, `cf.`,
   `vs.`, `viz.`, `incl.`, `esp.`, `approx.`, `resp.`, `et al.`, `Fig.`, `Eq.`. Decimals
   (`0.8`), section numbers (`§4.5.2`) and dotted ids never split because no space follows
   the dot.
4. **Captions.** A sentence that is only a bold span (`**Requirement.**`,
   `**Hardware handling of …** (§3.3).`) joins the next sentence, unless the bold span
   holds a finite verb or modal (*is, are, was, were, has, have, does, do, must, may, shall,
   cannot, should*, either case): `**Regions are threaded the same way.** **Indexed
   operand** …` is two sentences (a sentence that ends inside bold, `.**` followed by space
   and a capital or a bold opener, is a boundary like any other). Only the bold span is
   tested, so `**Lifted width rules** (… a mismatch is ill-formed …).` stays a caption.
5. **Markers.** A `[deps: …]` marker ends its sentence (with any `.` right after it).
6. **Enumerations.** Inside a sentence, a run of at least two consecutive enumerators
   starting at `(1)`, `(a)` or `(i)` yields sub-statements. An enumerator preceded by a
   `§` reference or by *part, item, step, rule, case, in, of, see, by, from, per* is a
   reference (`§9.2 (5b)`, `replaced in part (4)`), not an item.

### Classification rules

First match wins. The rule that fired is stored per statement (`rule`), so every count
below can be traced to a rule.

Evaluation order: K1/K12, K2, K3, K4, K5/K13, K6, K7, K8, K9, K14, K16, K15, K10 (K11 for
sub-statements). Rules K12–K16 were added on 2026-10-31 for the proof retag's
`proofs/STATEMENT-NOTES.md`; K10 and K11 keep their numbers so that earlier references
stay valid (the normative-fence rule is therefore K12, not K10).

| Rule | Kind | Condition |
|---|---|---|
| K1 | example | a fenced code block for which the K12 predicate is false (below): it is an example or witness |
| K12 | requirement, or definition when its introducing statement is a definition | a fenced code block that is normative: see the predicate below |
| K2 | non-normative | its section or an enclosing one is front matter, a change log (title `Changes from …`), or titled `(non-normative)` / `(deferred)` |
| K3 | non-normative | it starts with `*(non-normative…)*` / `*(deferred…)*`; the marker scopes its own sentence only (before 2026-10-31 it ran to the paragraph end and swallowed rules such as `ISA-6.4-P1.S4` and the §12.1 positivity test of the 0.9 draft); **uncertain** if it contains an RFC 2119 keyword |
| K4 | requirement | an upper-case RFC 2119 keyword outside code spans and inline `*(non-normative: …)*` asides: MUST, MUST NOT, SHOULD, SHOULD NOT, MAY, SHALL, REQUIRED, RECOMMENDED, OPTIONAL |
| K5 | definition | it carries a defined term or rule label of its clause (the first statement whose bold span is one of the clause's term aliases or ambiguous terms, or that names one of its labels; `defines` records them) **and** introduces it as the subject being defined: `**X** is/are/denotes/means/refers to/stands for/consists of …`, `**X** = / := / ≡ …`, `**X**: <noun phrase>` (the part after the colon starts with an article or has no declarative verb), `**X** (…):` closing the sentence (it introduces a table or list), or `… is/are/called/named the **X**`; an article (*a, an, the, its, each, every*) may precede X and up to eight verbless words, code spans or parentheticals may follow it. Also: a term (not a label) whose sentence is otherwise verbless (`**Tier.** CC, R, Q.`), and a verbless row of an id table (`DP | Platform | the platform profile`) |
| K13 | requirement | it carries a term or label (as for K5) that it does not introduce, i.e. a run-in caption `**X.** <rule>`, an inline bold term (`… only inside **borrow frames**: …`) or a label row, and the rest states a rule (the K7 or K8 test); or a label caption outside a table heads a verbless remainder (`**T5.** As §6.3.`). `defines` still records the term, so term uses resolve to it |
| K6 | example | starts with *e.g.* / *For example* / *For instance* / *Example*, contains *e.g.*, or lies in an examples section (title `Example…`, `… Examples`, `Worked examples`, unless marked `(normative …)`) |
| K7 | requirement | a lower-case normative word: *must, must not, cannot, shall, may only, may not, never* |
| K8 | requirement | a declarative verb from a fixed list (*is, are, has, returns, rejects, traps, faults, holds, produces, …*; about 250 forms, `DECL_VERBS`) |
| K9 | requirement | a table row in a normative section (instruction, default and error-code rows state their content in columns) |
| K14 | definition | a verbless equation: `X = …`, `X := …`, `X ::= …` with X a code span or up to three words, a code span opening with such an equation (`` `NAME := …` ``), or any ` ≡ ` |
| K16 | (previous) | a verbless fragment that follows a requirement or definition of its clause (`Predicate stack empty at entry and exit.`, `No fault outcome.`), or opens a list item whose previous clause in the section ends in one (items continuing a stem): it takes that statement's kind |
| K15 | requirement | a verbless colon-labelled fragment (`Forms: …`, `**Body**: …`, `Default: 1.`, `Successor: …`; at most four words before the colon, after an optional bold caption) or a bare colon introduction (`Frames:`): field and list stems of a normative text |
| K10 | uncertain | none of the above: verbless fragments with no normative neighbour (`Byte-addressed, little-endian, sparse.` opening `ISA-7.1-P1`, `F ≥ 4.`), inline JSON schemas, the §16.1 lexical rules |
| K11 | (parent's) | a sub-statement that would be K10 inherits its enumerating sentence's kind |

**K12 predicate.** A fenced code block of a clause in section S is an example (K1) if and
only if one of:

1. S or an enclosing section is non-normative (K2 titles) or an examples section (K6 titles);
2. the fence's info string carries a `key=value` attribute (regex `(^|\s)[A-Za-z][\w.-]*=`):
   the machine-checked witnesses of the dialect spec (`llvm example=…`, `text state=…`,
   `text run=…`, `isa object=…`, …);
3. S is not inside an examples section titled `(normative …)` (ISA §16.3 *Example objects
   (normative test vectors, suite C and F)*), and some preceding top-level statement of the
   same clause is a `*(non-normative)*` sentence, is classified `example`, or contains
   *e.g., for example, for instance, example(s), witness(es), sample(s), illustrat…*
   (case-insensitive, code spans and inline asides removed).

Otherwise it is normative (K12): a `definition` when the nearest preceding prose statement of
its clause is a definition (`**Adjoint table** (…):`, `**Semantic envelope** of a symbol
(…):`, `**Schedule document** (JCS):`), else a `requirement` (including fences that open their
clause straight after a heading, which have no introducing statement).

Fences reclassified from `example` to normative by K12 (old id = new id in every case):

- ISA 0.8 (20 of 20): requirement `ISA-3.1-P1.S1`, `ISA-3.3-P1.S1` (type list),
  `ISA-4.4-P1.S1` (gate unitaries), `ISA-5.4-P1.S4` (fault codes), `ISA-8.1-P1.S3`,
  `ISA-8.2-P1.S1`, `ISA-8.3-P1.S1`, `ISA-9.1-P1.S3` (section schemas), `ISA-10-P7.S2`,
  `ISA-11-P1.S1`, `ISA-12.3-P1.S1`, `ISA-16.2-P1.S1` (object grammar), `ISA-16.3-P1.S3`,
  `ISA-16.3-P2.S2`, `ISA-16.3-P4.S2`, `ISA-16.3-P5.S7`, `ISA-16.3-P6.S2`; definition
  `ISA-5.3-P4.S2` (Adjoint table), `ISA-9.2-P1.S3` (envelope parts), `ISA-11-P3.S2`
  (schedule document).
- ISA 0.9 draft (31 of 40): requirement `ISA-3.1-P1.S1`, `ISA-3.3-P1.S1`, `ISA-3.7-P2.S2`,
  `ISA-4.4-P1.S1`, `ISA-5.4-P3.S7`, `ISA-8.1-P1.S3`, `ISA-8.2-P1.S1`, `ISA-8.3-P1.S1`,
  `ISA-9.1-P1.S3`, `ISA-9.4-P4.S4`, `ISA-10-P7.S2`, `ISA-11-P1.S1`, `ISA-11.1-P3.S2`,
  `ISA-12.3-P1.S1`, `ISA-16.2-P1.S1`, `ISA-16.3-P2.S3`, `ISA-16.3-P3.S2`, `ISA-16.3-P6.S7`,
  `ISA-16.3-P7.S2`, `ISA-16.3-P22.S4`; definition `ISA-5.3-P4.S2`, `ISA-9.2-P2.S3`,
  `ISA-11-P3.S2`, `ISA-16.3-P5.S2`, `ISA-16.3-P8.S2`, `ISA-16.3-P10.S2`, `ISA-16.3-P12.S2`,
  `ISA-16.3-P14.S2`, `ISA-16.3-P16.S2`, `ISA-16.3-P18.S2`, `ISA-16.3-P20.S2`. The 9 that
  stay examples have example wording in their clause outside §16.3.
- DLT 0.2.10 (6 of 501): requirement `DLT-3.1-P6.S2`, `DLT-3.2-P1.S2` (`markers-ir`),
  `DLT-3.3-P1.S3` (`markers-c`), `DLT-8.6-P4.S2`, `DLT-19.1-P1.S4`; definition
  `DLT-9.2-P2.S2`. The other 495 carry a witness attribute.

K4 precedes K5: a sentence that introduces a term *and* states a MUST is a requirement
(its `defines` still records the term, so term uses resolve to it). K5/K13 split the old K5
(which made every statement carrying a bold term a definition): `ISA-5.1-P1.S5`/`.S6`,
`ISA-5.6-P2.S1`, the theorems T1–T6 of §14.3 and the closure-set cases of `ISA-9.2-P2` are
requirements that carry their term or label; `**Interface regions** are caller-owned …` and
`**Budget**: classical (…)` stay definitions.

### Dependencies

**Inferred** (`deps.inferred`, `via` = `term` | `label` | `refs` | `xspec`): the same
scanners as the clause graph, run on each statement's own text. A term or label mention
points at the *defining statement* (the K5 placement above; a label whose defining clause
has no statement naming it falls back to `.S1`); a `§` reference points at the section or,
for item references (`§9.2 (5b)`), the clause; `ISA §…` in the dialect is resolved against
the `--xspec` graph. A statement never depends on itself or its enumerating parent.

**Declared** (`deps.declared`): the 0.9 draft editor writes an optional marker at the end
of a sentence, after or before its final period:

```
… and its recipe MUST reproduce it exactly. [deps: ISA-4.5-P5, ISA-11.1-P21]
```

Entries are comma-separated statement ids (`ISA-3.3-P2.S1`), clause ids, section ids
(`ISA-5.6`), aliases (`PT3`, `ISA:lproc`), or ids of the cross-spec graph; `[deps: none]`
declares that the sentence has no dependencies. Declared edges take precedence: once a
sentence carries a marker, each inferred edge is `confirmed` (its target is, or lies
inside, a declared target) or `superseded` (not followed by queries, shown with `~` in the
`.md`); without a marker inferred edges have status `inferred`. A marker entry that does
not resolve is listed in `unresolved` with a reason (`no-such-id`, `no-such-statement`,
`unknown-alias`, `xspec-not-loaded`, `self`) and printed as a warning; it never fails the
build. The marker text is excluded from inference (its ids are not "mentions").

The current 0.9 draft (sha256 e98efc52c9be) already carries 8 markers (§0 change K28 and
§11.1's instruction-release paragraphs): 19 declared edges, all resolved, 2 inferred edges
superseded (a `§4.5` reference where the marker names `ISA-4.5-P5`; a `K28` mention).

### Results

| Graph | statements | top-level | sub | requirement | definition | non-normative | example | uncertain | inferred edges | declared markers (edges) |
|---|---|---|---|---|---|---|---|---|---|---|
| ISA-0.8 | 541 | 533 | 8 | 450 | 73 | 11 | 1 | 6 | 286 | 0 |
| ISA-0.9-draft | 1397 | 1389 | 8 | 1140 | 93 | 136 | 11 | 17 | 1050 | 8 (19) |
| DLT-0.2.10 | 2841 | 2835 | 6 | 1859 | 135 | 82 | 725 | 40 | 2881 | 0 |

Before the 2026-10-31 fixes (K12–K16, caption verbs, K3 scope) the same graphs gave
ISA-0.8 540: requirement 359, definition 101, non-normative 13, example 21, uncertain 46;
ISA-0.9-draft 1396: 972 / 142 / 146 / 42 / 94; DLT-0.2.10 2840: 1557 / 290 / 113 / 709 /
171. A sentence ending inside bold now splits once per graph: `ISA-3.3-P2.S2` in 0.8 (its second
sentence is now `ISA-3.3-P2.S3`), `ISA-3.3-P3.S2` in the 0.9 draft (now `.S3`), and
`DLT-4.3-P1.S3` (now `.S4`; `.S4`/`.S5` renumber to `.S5`/`.S6`). No other id moved.

By rule (counts include sub-statements):

| Rule | K1 | K2 | K3 | K4 | K5 | K6 | K7 | K8 | K9 | K10 | K11 | K12 | K13 | K14 | K15 | K16 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ISA-0.8 | 0 | 8 | 3 | 23 | 54 | 1 | 9 | 229 | 102 | 6 | 1 | 20 | 46 | 8 | 8 | 23 |
| ISA-0.9-draft | 9 | 130 | 6 | 72 | 67 | 2 | 63 | 742 | 106 | 17 | 1 | 31 | 76 | 8 | 10 | 57 |
| DLT-0.2.10 | 495 | 39 | 43 | 42 | 129 | 230 | 95 | 1218 | 214 | 40 | 2 | 6 | 158 | 1 | 0 | 129 |

Uncertain: 6 (1.1 %), 17 (1.2 %), 40 (1.4 %), down from 46 (8.5 %), 94 (6.7 %), 171 (6.0 %):
K14–K16 decide equations, colon stems and fragments next to a normative sentence; what is
left has no normative neighbour (ISA 0.8: `ISA-1.1-P1.S1`, `ISA-5.4-P2.S1` `F ≥ 4.`,
`ISA-7.1-P1.S1`, the inline JSON schemas `ISA-12.2-P1.S1` and `ISA-13-P1.S1`, the §16.1
lexical rules `ISA-16.1-P1.S1`). Before rule K9 existed, bare table rows were the largest
uncertain group (124 of 190 in ISA 0.8); they are counted separately under K9 so that the
choice stays visible. K3 covers 3 / 6 / 43 statements (the marker sentences themselves),
none uncertain.

Kinds per top-level section, ISA 0.8 (per (sub)section counts for all three specs are at
the top of each `.statements.md`):

| § | front | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 12 | 13 | 14 | 15 | 16 | 17 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| requirement | 0 | 9 | 0 | 92 | 92 | 40 | 26 | 39 | 28 | 21 | 19 | 16 | 21 | 0 | 10 | 8 | 16 | 13 |
| definition | 0 | 0 | 14 | 22 | 2 | 7 | 0 | 2 | 0 | 9 | 1 | 2 | 3 | 0 | 0 | 0 | 4 | 7 |
| non-normative | 8 | 0 | 0 | 1 | 0 | 0 | 1 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| example | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| uncertain | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 1 | 0 | 0 | 0 | 0 | 1 | 1 | 0 | 0 | 1 | 0 |

ISA §5.1's single paragraph `ISA-5.1-P1` becomes seven statements:

```
ISA-5.1-P1
  <1>1 [req] Signatures list roles: `oracle @o (in a: wire[8]) ... (args n: i64)`.
  <1>2 [def] **Interface regions** are caller-owned, passed by name, role `in` (...) or `mut` (actions only).
  <1>3 [req] A region MUST NOT contain `measure effect br call uncall trap hostcall qreg.*` or classical memory instructions, nor lifted rows outside oracle bodies.
  <1>4 [req] Predicate stack empty at entry and exit.                       (K16)
  <1>5 [req] Every `qalloc` matched by a `qfree` of a clean bundle, ..., by **explicit cleanup instructions**; the schedule chooses only realisations.   deps: ISA-2-P1.S2 (bundle)
  <1>6 [req] **`in` wires in actions** appear only as controls, ...          (K13)
  <1>7 [req] Action call graphs are acyclic; recursion exists only through `lift.call`.
```

### Queries on statements

`impact`, `independent`, `coverage` and `diff` take `--statements`, which loads the
sibling `<graph>.statements.json` of every loaded graph (required for the first, optional
for `--with` graphs); a statement id argument (or tag) implies it. With the layer loaded,
a spec's statement edges *replace* its clause edges (they are a refinement of them);
statement nodes are children of their clause (sub-statements of their parent), so the
semantics of "Dependency semantics" carry over unchanged: citing a clause or section cites
every statement in it. A clause id therefore falls back to the union of its statements.

```
impact clauses/ISA-0.8.json ISA-5.1-P1.S3 --with clauses/DLT-0.2.10.json
impact of ISA-5.1-P1.S3 [req] (closure anchored): 14 dependents (14 statements, 0 clauses without a statement layer; 13 clauses in all)
depth 1 (8): DLT-1.3-P1.S2 [req], DLT-6.8-P1.S2 [req], DLT-8.2-P3.S1 [req], DLT-8.2-P4.S1 [req], ISA-1.2-P1.S1 [req], ISA-1.2-P2.S1 [req], ISA-3.2-P6.S1 [req], ISA-14.3-P6.S1 [def]
depth 2 (5): ISA-4.2-P29.S2 [req], ISA-5.7-P1.S1 [req], ISA-11-P2.S3 [def], ISA-12.3-P2.S1 [req], ISA-15-P3.S1 [req]
depth 3 (1): ISA-11-P2.S1 [def]
```

`independent` keeps the strict definition by default: two statements of `ISA-5.1-P1` share
the ten statements that cite `§5.1` or `§5` and are reported dependent. `--containers
ignore` discounts citations of a section or clause that encloses *both* arguments (such a
citation cannot tell them apart); with it, `ISA-5.1-P1.S3` and `ISA-5.1-P1.S7` are
independent, which is the statement-granularity answer the layer exists for.

`coverage --statements` reports, besides the clause table, each statement as **strong**
(the statement or its enumerating parent is tagged), **weak** (only its clause or a
section is tagged: a clause tag is weak coverage of its statements) or **uncovered**, in
total and per kind. A statement tag also counts as a strong witness of its clause in the
clause-level report.

**Proof coverage** (`coverage clauses/ISA-0.8.json --tags proofs/tags.json --statements`,
2026-10-31, proofs citing statement ids, after K12–K16): clauses strong 76, weak 9,
uncovered 229 of 314; **statements of ISA-0.8: 541; strong 109, weak 11, uncovered 421**;
requirements 450 (strong 98, weak 10, uncovered 342), definitions 73 (strong 11, weak 1,
uncovered 61), non-normative 11, example 1 and uncertain 6 all uncovered. (With clause-id
tags the same proofs gave strong 0, weak 159.)

`diff --statements` matches statements by hash (repeats in document order), then by
`difflib` ratio ≥ 0.6 between the remaining statements of clause pairs that the clause
diff matched; the rest are added or removed. ISA 0.8 → 0.9-draft (e98efc52c9be):

```
statements ISA-0.8 -> ISA-0.9-draft: 540 -> 1396; matched 506 {"hash": 423, "similar": 83}; unchanged 276; moved 185 (renumbered only 147); changed 83; added 890; removed 34
by kind: added {"definition": 44, "example": 23, "non-normative": 130, "requirement": 645, "uncertain": 48}; removed {"definition": 3, "example": 2, "non-normative": 1, "requirement": 28}; changed {"definition": 11, "example": 13, "non-normative": 5, "requirement": 48, "uncertain": 6}
```

The 80 clauses the clause diff calls changed hold 214 statements in 0.8: 120 of them are
hash-identical in 0.9, 83 changed, 11 removed, and 56 new statements were added into those
clauses. A council finding or proof citing one of the 120 is unaffected by the revision of
its paragraph.

### Limits of the statement layer

- **Lower-case sentence starts are not boundaries.** `… location arguments. flag := …`,
  `… dematerialised first. len=0 changes nothing` stay one statement (a rough scan finds 2
  such statements in 0.9 and 8 in the dialect); writing the identifier in backticks fixes it.
- **Table rows are not split into cells.** A row is one statement unless a cell holds
  several sentences; the dialect's Appendix C rows (`DLT-C-P14.S1`, 3247 characters)
  remain single statements, as do 9 dialect and 5 ISA 0.9 rows over 700 characters.
- **Definitions inherit the clause graph's term detector.** A bold run-in caption without a
  period (`**Lifted width rules**`, `**Meaning in brief**`) is a term alias there, so its
  sentence becomes K5 `definition` even when it states requirements without an RFC
  keyword (18 / 27 / 17 statements define a term of three or more words in ISA 0.8 / 0.9 /
  dialect; most are such captions).
- **Fragments.** List items that continue a stem from the previous clause (`budgets (§9);`,
  `a marker (§3.2) or a primitive (§3.1);`) and colon introductions (`It fixes:`,
  `Frames:`) are K10 uncertain: 93 of the dialect's 171 uncertain statements are list
  items, 34 of them starting in lower case; 15 end in `:`.
- **K8 and K9 are heuristics.** The verb list contains noun-ambiguous forms (*uses, names,
  lists, maps, records*), and K9 calls every normative table row a requirement. Both are
  separate rules so that consumers can treat them as weaker than K4.
- **Scope of `*(non-normative)*`** is assumed to be the rest of its paragraph (K3). Where
  the spec returns to rules inside that paragraph without an RFC keyword, K3 is wrong:
  `ISA-3.6-P17.S3` (`B = 0 is legal: the oracle commits FAULT_BUDGET_STEPS …`) follows a
  non-normative note and is classed non-normative. With a keyword it is flagged uncertain.
- **One level of enumeration**; enumerations spread over list items are already separate
  clauses. Enumerations restarting at `(1)` inside an item are not nested further.
- **Positional ids** (as for clauses): inserting a sentence renumbers the clause's later
  statements; use aliases, statement hashes and `diff --statements --list` to re-target.

## Limits

- **Clause ordinals are positional.** Inserting a paragraph renumbers the rest of its
  section; ids are stable only within one spec revision. `diff` is the bridge between
  revisions; anchors in the spec text would be better but accepted bytes are not edited.
- **Code runs.** All code blocks between two prose blocks join the clause before them,
  so example-heavy dialect sections produce large clauses (`DLT-12.8-P27` spans 213
  lines and 6 code blocks).
- **Table headers are not hashed.** A header change does not change row hashes.
- **Section granularity.** Most references cite whole sections; a clause citing `§12`
  depends on all of §12. This is the cause of the full-closure saturation above; the
  anchored closure is a deliberate approximation, not a proof of independence.
- **Labels defined only in running prose are not found.** The dialect's rung minis
  `R1`–`R8` (§21.4: "R1 (an Int8 Rust function …), R2 (a C hashtable …)") have no table,
  bold or list form; `R2` resolves in the ISA (§3.7 return step), not in the dialect.
  Ranges written with `to` (`TDL-1 to TDL-12`) are not expanded; en-dash ranges are.
- **Single-letter labels** (ISA suites `A`–`H`, change entry `V`) are not aliases, so
  "suite F" mentions create no edge.
- **Terms are matched literally.** Case-sensitive: `Interface regions` (bolded at a
  sentence start) does not match `interface regions`; only `s`/`es` plurals are
  folded; one-word plain terms outside Terminology are dropped (precision over recall);
  ambiguous terms are dropped (listed in the JSON). Bold emphasis of a multi-word phrase
  that is not a definition (`exact-permutation only`) still becomes a term; it is rarely
  repeated, so it rarely produces edges.
- **Inline enumerators are heuristic.** `(a)` in running text counts as a marker; item
  references prefer list-item markers, so the heuristic only decides when no list item
  matches.
- **Headings carry no references.** No heading in the three specs contains `§`; one that
  did would contribute no edge.
- **External documents** (north star, SQLite plan, spike findings, outlines, prior
  versions) are recorded as unresolved by design. Building the north star into a graph
  would turn the 105 dialect `NS §` references into edges.
- **The 0.9 draft moves.** Its graph describes the revision whose sha256 it records.
- **No SVG.** Graphviz is not installed on the build machine; `build --svg` reports that.
