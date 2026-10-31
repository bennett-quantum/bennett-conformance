# INV-4: the identity envelope is injective on structural layout leaves

Spec: ISA 0.8 (`clauses/ISA-0.8.json`). Format: `proofs/FORMAT.md`.

## Informal statement

A symbol's closure hash is SHA-256 of its semantic envelope, and part (5b) of the
envelope lists the layout leaves in the symbol's closure set, one line per leaf,
sorted by dotted path. For the hash to identify what the symbol depends on, the
(5b) block must be a function of the closure set (two conforming implementations
produce the same bytes) and an injective one (different closure sets give
different bytes). **Both properties fail in ISA 0.8**, for one family of leaves
only: `interface_regions.<oracle>.<region>`, because oracle and region names may
contain `.`, so the leaves ("a", "c.db") and ("a.c", "db") render to the same
dotted path. Below we prove both properties for every other leaf family and for
every symbol kind whose closure set cannot contain two such leaves, and record
the failure as a refuted step with the sentence 0.9 needs.

## Precise statement

DEFINE for a legal `layout` object L (`ISA-9.1-P1.S1`, `ISA-9.1-P1.S3`): `Leaves(L)` = the set of
pairs (k, v) where k = (k_1, …, k_m) is the tuple of JSON keys from the root to
a leaf at "the deepest level the schema lists" and v its value (`ISA-9.2-P2.S7`).
DEFINE `dot(k)` = k_1 "." … "." k_m. DEFINE `line(k, v)` = "layout " dot(k) " "
JCS(v) LF. DEFINE for a symbol f: `CS(f) ⊆ Leaves(L)` its closure-set leaves by
kind (`ISA-9.2-P2.S1`, `ISA-9.2-P2.S2`, `ISA-9.2-P2.S3`,
`ISA-9.2-P2.S4`, `ISA-9.2-P2.S5`, `ISA-9.2-P2.S6`), and `E5b(f)` = the concatenation of line(ℓ), ℓ ∈ CS(f),
"sorted by dotted path bytewise" (`ISA-9.2-P1.S3`).

THEOREM INV-4 (as intended). For every symbol kind K and all legal objects O,
O' defining a symbol f of kind K:
- (W) `E5b(f)` is uniquely determined by `CS(f)`;
- (I) `E5b_O(f) = E5b_O'(f)` implies `CS_O(f) = CS_O'(f)`.

THEOREM INV-4' (what holds in 0.8). (W) and (I) hold whenever dot is injective
on `CS(f)` (on `CS_O(f) ∪ CS_O'(f)` for (I)); dot is injective on every leaf
family except `interface_regions.<oracle>.<region>`; hence (W) and (I) hold for
oracle, lproc, action, permutation-action and data symbols, and fail for CC/R
symbols.

## Hypotheses

ISA-9.1-P1.S1, ISA-9.1-P1.S3, ISA-9.1-P2.S1, ISA-9.2-P1.S3, ISA-9.2-P2.S1,
ISA-9.2-P2.S2, ISA-9.2-P2.S3, ISA-9.2-P2.S4, ISA-9.2-P2.S5, ISA-9.2-P2.S6,
ISA-9.2-P2.S7, ISA-16.1-P1.S2, ISA-16.1-P1.S3, ISA-16.2-P1.S1.

## Proof

```
<1>1. line is injective on leaves: line(k, v) determines (dot(k), v).
      PROOF:
  <2>1. dot(k) contains no space and no LF: schema keys are fixed identifiers
        and every free key is a NAME, `[A-Za-z_][A-Za-z0-9_.]*`.
        BY ISA-9.1-P1.S3, ISA-16.1-P1.S2
  <2>2. JCS(v) contains no raw LF (RFC 8785 escapes control characters in
        strings and emits no insignificant whitespace); for a scalar the value
        is "its decimal text", fixed per field because "Integers that may
        exceed 2^53 are decimal strings" is a per-field rule.
        BY ISA-16.1-P1.S3, ISA-9.1-P2.S1, ISA-9.2-P2.S7
  <2>3. Hence the first space after the fixed prefix "layout " ends dot(k),
        the rest up to LF is JCS(v), and JCS is a canonical serialisation, so
        v is recovered.
        BY <2>1, <2>2, ISA-16.1-P1.S3
  <2>4. QED
        BY <2>3
<1>2. dot is injective on each leaf family except interface_regions, and
      across families.
      PROOF:
  <2>1. The families are: top-level scalars (`ptr_width`, `H`, `A_tab`,
        `S_cells`, `null_size`, `heap_base`, `heap_limit_max`, `block_image`);
        `globals.<name>`; `interface_regions.<oracle>.<region>`. "Unknown
        sections or fields are errors".
        BY ISA-9.1-P1.S1, ISA-9.1-P1.S3, ISA-9.2-P2.S7
  <2>2. Top-level scalar paths are fixed, dot-free and pairwise distinct.
        BY ISA-9.1-P1.S3
  <2>3. `globals.<name>` has one free component behind a fixed prefix, so
        dot(k) = "globals." ‖ name determines name.
        BY ISA-9.1-P1.S3, ISA-16.1-P1.S2; algebra
  <2>4. Paths of different families differ in their first component, which
        is dot-free ("globals", "interface_regions", the scalar names), so the
        prefix up to the first "." identifies the family.
        BY <2>1, <2>2
  <2>5. `interface_regions.<oracle>.<region>` has two free components, each a
        NAME that may contain ".": ("a", "c.db") and ("a.c", "db") both give
        "interface_regions.a.c.db". dot is not injective on this family.
        BY ISA-9.1-P1.S3, ISA-9.2-P2.S7, ISA-16.1-P1.S2
  <2>6. QED
        BY <2>2, <2>3, <2>4, <2>5
<1>3. If dot is injective on CS(f), then (W) and (I) hold.
      PROOF:
  <2>1. Sorting by an injective key is a total order on CS(f), so E5b(f) is
        unique.
        BY ISA-9.2-P1.S3; algebra
  <2>2. E5b(f) splits at LF into lines; by <1>1 each line gives (dot(k), v);
        by injectivity of dot it gives k; hence CS(f).
        BY <1>1, <2>1; algebra
  <2>3. QED
        BY <2>1, <2>2
<1>4. dot is injective on CS(f) for f an oracle, lproc, action, permutation
      action or data symbol.
      PROOF:
  <2>1. Oracle: the closure set has all layout leaves "except
        `interface_regions` entries of other oracles", plus "its own
        `interface_regions.<oracle>.*` leaves"; all interface-region leaves in
        it share the first free component (the oracle), so only the region
        component is free.
        BY ISA-9.2-P2.S2, <1>2
  <2>2. "Its own" and "other oracles" are decided on the structural key
        k_2 = the oracle's name, not on a prefix of the rendered path.
        ASSUMED: `ISA-9.2-P2.S2` writes the selection as the path pattern
        `interface_regions.<oracle>.*`; on rendered paths, oracle "a" would
        capture the leaves of oracle "a.b" (council SOL-R11-B02, R11-B02).
  <2>3. Lproc: only `globals.<g>` leaves (and images); action, permutation
        action, data: no layout leaves.
        BY ISA-9.2-P2.S3, ISA-9.2-P2.S4, ISA-9.2-P2.S6, <1>2
  <2>4. QED
        BY <2>1, <2>2, <2>3, <1>2
<1>5. For CC and R functions, (W) and (I) fail.
      PROOF:
  <2>1. A CC/R closure set has "all `layout` leaves", so it contains every
        interface-region leaf of every oracle.
        BY ISA-9.2-P2.S5
  <2>2. (W) fails: an object with oracle "a" (region "c.db", base 32) and
        oracle "a.c" (region "db", base 48) is legal and gives a CC function
        two lines with the same dotted path; "sorted by dotted path bytewise"
        does not order them, so two envelopes (and closure hashes) conform.
        OPEN (refuted): `ISA-9.2-P1.S3` (5b) gives no tie order; council
        SOL-R12-B01, R12-B01; SPEC-QUESTIONS 31.
  <2>3. (I) fails even with any tie order: object O1 with oracle "a", region
        "c.db" and object O2 with oracle "a.c", region "db", both base 32, give
        a reference-free CC function with identical text identical (5b) lines,
        hence identical envelopes, while CS_O1 ≠ CS_O2. With a value-based tie
        order (sorting whole lines), O1 = {a/c.db: 32, a.c/db: 48} and
        O2 = {a/c.db: 48, a.c/db: 32} collide too.
        OPEN (refuted): the tie-break of SPEC-QUESTIONS 31 (by original key
        components) repairs (W) but not (I), because the line does not carry
        the components.
  <2>4. Whether a CC function's behaviour can depend on the lost distinction
        (it can address an interface region's `base` in classical memory
        without naming the oracle) is not decided here.
        OPEN: harm assessment; part (6) separates the two only when the
        function references the oracles.
  <2>5. QED
        BY <2>2, <2>3
<1>6. QED
      BY <1>3, <1>4 for INV-4' on the listed kinds; <1>5 refutes INV-4 for
      CC/R.
```

Not covered: injectivity of the whole envelope (parts (1)–(6) as one byte
string). It needs a grammar argument over `ISA-16.2-P1.S1` that no canonical function
line has the form of a (5a)/(5b)/(5c) line or of a 64-hex part-(6) line, so that
the part boundaries are recoverable. OPEN; listed as gap 4.

## Witnesses

| Witness | Steps |
|---|---|
| `suite/cases/sum_db`, `heap` (one oracle, one interface region, globals: non-colliding leaves; pinned closure hashes) | 1.3, 1.4 |
| `suite/cases/control_flow`, `classical`, `tiny_memory`, `wide`, `mutual` (CC/R and oracle symbols with layout leaves, no collisions) | 1.3 |
| `tests/identity.rs`: `r11_oracle_layout_ownership_uses_exact_names`, `r11_dotted_oracle_layout_ownership`, `r11_dotted_owner_and_region_family`; `tests/cli_xam.rs::r11_cli_exact_oracle_layout_owners` | 1.4.2 (ASSUMED) |
| `tests/identity.rs`: `r12_cc_r_colliding_dotted_leaves`, `r12_cc_r_layout_paths_preserve_colliding_leaves`, `r12_dotted_leaf_collision_family` | 1.5.2 (pins the SPEC-QUESTIONS 31 tie order) |
| `tests/identity.rs::r12_dotted_global_lproc_keeps_exact_closure_selection` | 1.2.3, 1.4.3 |

No existing witness exercises 1.5.3 (cross-object collision): every current test
compares envelopes within one object.

## Council blockers as instances

- SOL-R11-B02, R11-B02 (oracle "a" owns the leaves of oracle "a.b" by rendered
  prefix): violation of 1.4.2; the selection must be structural.
- SOL-R12-B01, R12-B01 (CC/R envelope coalesces `interface_regions["a"]["c.db"]`
  and `interface_regions["a.c"]["db"]` into one map key, dropping a leaf): an
  implementation choosing a lossy representation because 1.2.5 makes the
  rendered path a non-key; the spec-level failure is 1.5.2.

## Spec gaps

1. (1.4.2, next to `ISA-9.2-P2` (`ISA-9.2-P2.S2`)) ASSUMED. Needed sentence: "An oracle's own
   interface-region leaves, and the interface-region entries of other oracles,
   are selected by the exact JSON key under `interface_regions`, never by a
   prefix of the dotted path."
2. (1.5.2, next to `ISA-9.2-P1` (`ISA-9.2-P1.S3`) part (5b)) OPEN (refuted). Needed sentence
   (SPEC-QUESTIONS 31, sol's wording): "Emit one line for every structured layout
   leaf, including leaves with coincident dotted renderings. Sort by the rendered
   dotted path bytewise; break ties bytewise by the sequence of original JSON
   key components."
3. (1.5.3, next to `ISA-9.2-P1` (`ISA-9.2-P1.S3`) part (5b) and
   `ISA-16.1-P1` (`ISA-16.1-P1.S2`)) OPEN (refuted); gap 2
   does not close it. Needed sentence, one of: (a) "The path of a layout leaf is
   rendered as the JCS array of its key components, e.g. `layout
   ["interface_regions","a","c.db"] {"base":32}`"; or (b) "Oracle names and
   region names that appear as keys of `layout.interface_regions` MUST NOT
   contain `.`" (a narrowing of NAME, to be checked against the dialect's name
   mangling before choosing it).
4. (whole envelope, next to `ISA-9.2-P1` (`ISA-9.2-P1.S3`)) OPEN. Needed: either a lemma that
   canonical function lines (`ISA-16.2`, `ISA-16.2-P1.S1`) never begin with `data `, `layout `,
   `device ` and are never 64 hex digits, or a length prefix per part.
5. (1.5.4, next to `ISA-9.2-P2` (`ISA-9.2-P2.S5`)) OPEN. Not a missing sentence but an unfinished
   analysis: whether a CC/R function's semantics can depend on an interface
   region's `base` without a part-(6) reference to the oracle. Gap 3 makes the
   question moot.
