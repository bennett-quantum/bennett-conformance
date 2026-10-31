# INV-6: certificate labels are sound, evidence is exact, host limits are honest

Spec: ISA 0.8 (`clauses/ISA-0.8.json`). Format: `proofs/FORMAT.md`.

## Informal statement

An `exhaustive` claim (§12.1) compares a region R, on every element of a
`domain` (§12.2), with an expected program or table. A `sampled` claim replays
draws from a domain. §12.2 says: "A `proof` label requires the domain to equal
D; otherwise the claim is `evidence` with the covered fraction." The checker's
verdict feeds T3 (§14.3), borrow frames (§5.6) and composition (§12.3), so the
classifier that decides "the domain equals D" is load-bearing. INV-1 to INV-5
take it for granted. INV-3 1.5.2 assumes a T3 claim over the full domain and
does not prove that the label means what it says. INV-6 states and proves the
missing theorem:

- (A) **Label soundness.** If a conformant checker returns `Proof` for an
  exhaustive claim comparing R with an expected program Q (or a table), then
  the full output tuples agree on every element of R's full domain D. In
  particular, `Proof` implies that the enumerated domain equals D component by
  component. Components are identified by role and position (interface bundle
  (group, slot), region cell (group, slot, index), borrowed bundle k), never
  by name alone, and a `borrowed` entry covers no interface component.
- (B) **Evidence semantics.** A claim whose domain is a strict subset of D, or
  a `sampled` claim, yields at most `Evidence`, with the exact number of
  replayed elements. The verdict depends only on R and Q restricted to those
  elements. It licenses no conclusion about the rest of D.
- (C) **Resource honesty.** A checker that cannot materialise or enumerate a
  legal domain reports a host limitation. It does not abort, does not reject
  the domain as ill-formed, and does not label the claim `Proof` or `Evidence`.

Council rounds 21 and 22 on the reference interpreter found one violation of
(C) and one of (A) (see "Council blockers as instances"). The proof shows:

- (A) holds in 0.8 under readings, because §12.2 does not say what an entry's
  `name` refers to. It never says that coverage is by name. It never says that
  coverage is by component either: the text compares "the domain" with D and
  leaves the map from entries to components implicit. Once a `bundles` or
  `regions` name is read as naming an interface slot, and a `borrowed` entry
  as naming a borrowed system by position, the label is sound (1.5). A name-set
  classifier is unsound under every reading (1.5.5).
- (B) holds, with the exact count derived from the domain (1.6.3), except
  that the "covered fraction" of a sampled claim is undefined (1.6.5). The
  phrase "licenses no conclusion" is true of the label. It is refuted as a
  statement about mathematics: for bijective subjects, agreement on all but
  one element forces agreement on the last (1.6.8). That limits the theorem,
  not the spec.
- (C) has no basis in 0.8. The spec names no checker outcome for a legal
  claim that cannot be checked on the host (1.7.5). What 0.8 does fix is what
  a checker must not do: reject a legal domain (1.7.2) or issue a label for
  elements it did not replay (1.7.3).

## Precise statement

Fix a well-formed object O and a region R of O with closure hash h
(`ISA-9.2-P2.S15`). R is a lifted oracle (§5.2) or a permutation action
(§5.3). Fix a claim c in R's `claims` of kind `exhaustive` (domain d, expected
`{program: h_Q}` or `{table: T}`) or `sampled` (domain d, sampler σ).

DEFINE comp(R), the **components** of R, each with an identity and a width w ≥ 1:
- interface bundle: (g, j), with g the role group and j the slot index within g
  in declared order. The groups are `in`, `tgt`, `fault` for an oracle and `mut`,
  `in` for an action;
- interface region cell: (g, j, i) for cell i < 2^A of the j-th region of group
  g (`region[A,W]`, width W). The groups are `in` regions for an oracle and
  `mut` and `in` regions for an action;
- borrowed bundle: (`borrowed`, k), the k-th borrowed bundle of R.

DEFINE D(R) = Π_{γ ∈ comp(R)} {0, …, 2^{w_γ} − 1}, the set of basis
configurations (`ISA-2-P7.S1`). Workspace is at 0 and local regions are at
their image, so neither is a component.

DEFINE res(e), the **resolution** of a domain entry e, as the component (or,
for `cells: "all"`, the set of cells) the entry denotes (readings 1.2.2,
1.2.3). DEFINE ⟦d⟧ ⊆ D as the set of configurations x with x(res e) ∈
[lo_e, hi_e] for every entry e, and x(γ) = z_γ for every γ ∉ img(res). Here
z_γ is the fill value of an omitted component (reading 1.2.4).

DEFINE covers(d) ⟺ res is injective, img(res) = comp(R), and every entry has
lo = 0 and hi = 2^{w} − 1.

DEFINE tup_R(x) as the full output tuple of R on x: the final value of every
component, in comp(R) order (reading 1.4.2).

DEFINE a **checker outcome** as one of `Proof`, `Evidence(n)` (n replayed
elements), `Reject(rule)` (a stated rule of §12 is violated: ill-formed claim
or tuple mismatch) and `HostLimit(clause)` (reading 1.7.6). A checker is
**conformant** if it meets the §12.1 obligations and §12.2 under the readings
of this proof.

THEOREM INV-6. For every conformant checker K, every such c, and every R, Q:
- (A) K(c) = `Proof` ⇒ c is exhaustive, covers(d), and tup_R(x) = tup_Q(x)
  (resp. the row of T for x) for every x ∈ D. A `borrowed` entry is never in
  res⁻¹ of an interface component, whatever its name.
- (B) ¬covers(d) or c sampled ⇒ K(c) ∈ {`Evidence(n)`, `Reject`, `HostLimit`}.
  For exhaustive c, n = |⟦d⟧| = Π_e (hi_e − lo_e + 1) and the covered fraction
  is n / |D|. For sampled c, n = `count`. K(c) is a function of R and Q
  restricted to the replayed elements.
- (C) If d is legal and K cannot materialise an element of ⟦d⟧ or enumerate
  ⟦d⟧, then K(c) = `HostLimit(§12.2)`. K terminates normally and never returns
  `Reject` on a legality ground, `Proof` or `Evidence` for that claim.

THEOREM INV-6' (what holds in 0.8).
- (A) holds under readings 1.1.3 (action D), 1.2.2 to 1.2.5 (entry resolution,
  borrowed position, fill, duplicates), 1.4.2 to 1.4.4 and 1.4.6 (tuple,
  program correspondence, table order, oracle budget) and 1.5.1 (label is the
  checker's verdict). The name-set classifier is refuted outright (1.5.5).
- (B) holds for exhaustive claims. For sampled claims the fraction is
  undefined (1.6.5). "No conclusion outside ⟦d⟧" is refuted as a mathematical
  statement when |D ∖ ⟦d⟧| = 1 and both sides are bijections of D (1.6.8).
- (C) holds only as a reading (1.7.6). 0.8 has no outcome for it (1.7.5 OPEN).
  Its negative half, never reject a legal domain and never label unreplayed
  elements, is proved (1.7.2, 1.7.3).

## Hypotheses

ISA-1.2-P4.S1, ISA-2-P1.S2, ISA-2-P7.S1, ISA-3.3-P1.S1, ISA-3.3-P3.S2,
ISA-3.3-P14.S1, ISA-4.3-P2.S1, ISA-4.4-P1.S1, ISA-4.4-P2.S1, ISA-5.1-P1.S1,
ISA-5.1-P1.S2, ISA-5.2-P1.S1, ISA-5.2-P2.S1, ISA-5.2-P2.S2, ISA-5.2-P2.S3,
ISA-5.2-P4.S1, ISA-5.3-P1.S1, ISA-5.3-P1.S4, ISA-5.6-P1.S1, ISA-5.6-P2.S2,
ISA-9.2-P1.S1, ISA-9.2-P2.S15, ISA-9.4-P2.S1, ISA-9.4-P3.S1, ISA-12.1-P1.S1,
ISA-12.1-P2.S1, ISA-12.1-P5.S1, ISA-12.1-P7.S5, ISA-12.1-P8.S1,
ISA-12.2-P1.S1, ISA-12.2-P1.S2, ISA-12.3-P2.S3, ISA-14.1-P1.S1,
ISA-14.2-P1.S2, ISA-14.3-P3.S1.

## Proof

```
<1>1. D(R) is the product over comp(R) of full ranges, and components with
      different identities are different systems.
      PROOF:
  <2>1. For an oracle, D is "every value of each `ins`, of `tgt`, of `fr`, of
        every cell of every `in` region, of every borrowed bundle; `qalloc`ed
        wires at 0; local regions at image".
        BY ISA-5.2-P4.S1
  <2>2. An action body denotes a unitary U on "mut ⊗ ins ⊗ mem_mut ⊗
        (borrowed)", block-diagonal in `ins` and in `in`-region cells and the
        identity on borrowed systems; a permutation action's U is a
        permutation matrix.
        BY ISA-5.3-P1.S1, ISA-5.3-P1.S4
  <2>3. Reading adopted: an action's D is every value of each `mut` and `in`
        bundle, of every cell of every `mut` and `in` region and of every
        borrowed bundle, with workspace at 0 and local regions at image.
        ASSUMED: §5.3 defines no D, although T3, T6, §5.7 and §12.2 use it
        for actions; the factor list "mem_mut" omits `in` regions that the
        same sentence calls block-diagonal; the reading takes the oracle
        pattern of <2>1 and the full factor list of <2>2.
  <2>4. Interface components are identified by role and position:
        signatures list roles, interface bundles and regions are "passed in
        declared order", and the frame descriptor lists interface slots by
        (role, width, class). A region[A,W] slot holds 2^A cells of W bits.
        BY ISA-5.1-P1.S1, ISA-5.1-P1.S2, ISA-9.4-P2.S1, ISA-9.4-P3.S1,
           ISA-3.3-P1.S1, ISA-3.3-P14.S1
  <2>5. Distinct interface components are distinct wires: "A region's
        interface bundles and regions are pairwise disjoint."
        BY ISA-3.3-P3.S2
  <2>6. A borrowed bundle is a tensor factor of its own: `qborrow` gives "n
        wires in unknown state" and "introduces a factor ... possibly
        entangled with a reference system R", and the region's operation is
        U ⊗ I on it. No interface component is that factor.
        BY ISA-4.3-P2.S1, ISA-14.2-P1.S2, ISA-5.6-P1.S1
  <2>7. QED: D(R) = Π_{γ ∈ comp(R)} {0..2^{w_γ} − 1}, |D| = 2^{Σ w_γ}, and a
        basis configuration assigns each component independently.
        BY <2>1, <2>3, <2>4, <2>5, <2>6, ISA-2-P7.S1; algebra

<1>2. A domain d denotes the set ⟦d⟧ ⊆ D, and
      |⟦d⟧| = Π_e (hi_e − lo_e + 1).
      PROOF:
  <2>1. The domain lists `bundles` and `borrowed` entries {name, width, lo,
        hi} and `regions` entries {name, cells: "all" | [{index, lo, hi}]},
        "bounds inclusive, enumeration order as listed, last component
        fastest".
        BY ISA-12.2-P1.S1
  <2>2. Reading adopted: the name of a `bundles` entry denotes the interface
        bundle of R that has that local name in R's header, and the entry
        resolves to its identity (g, j). The name of a `regions` entry likewise
        resolves to an interface region (g, j). `cells: "all"` resolves to
        its 2^A cells (g, j, i), and a list entry to cell (g, j, index). A name
        that matches no interface bundle (resp. region) of R makes the claim
        ill-formed.
        ASSUMED: §12.2 gives the field `name` and nothing else. It says
        neither that names are R's header names nor what they resolve to.
        Header names are fixed by h (canonical function bytes, ISA-9.2-P1.S1),
        so the reading is well defined; the identity it yields is positional
        (<1>1.4).
  <2>3. Reading adopted: the k-th `borrowed` entry denotes the borrowed
        bundle (`borrowed`, k) of R, in the order of R's borrow requirement
        (first `qborrow` in canonical text order). Its `name` refers to
        nothing and is never looked up among interface names.
        ASSUMED: §12.2 does not say what a `borrowed` name refers to, nor
        that it may or may not coincide with an interface name. A borrowed
        system has no interface name to match (`w = qborrow cls n` binds
        only a body-local value, <1>1.6).
  <2>4. Reading adopted: a component of comp(R) that no entry resolves to
        holds one fixed value z_γ on every element that is replayed (the
        reference interpreter's choice is z_γ = 0).
        ASSUMED: §12.2 does not say what an omitted component holds, or
        whether a domain may omit one. The theorem depends on z only through
        ⟦d⟧ ⊆ D.
  <2>5. Reading adopted: an entry whose `width` differs from its component's
        width, an index ≥ 2^A, hi ≥ 2^w, lo > hi, or two entries that resolve
        to one component make the claim ill-formed (`Reject`).
        ASSUMED: §12.2 states none of these conditions.
  <2>6. QED: under <2>2 to <2>5 res is an injective partial map into comp(R)
        (cells counted singly), ⟦d⟧ is the product of the entry ranges with the
        omitted components fixed at z, and the enumeration of <2>1 lists each
        element once.
        BY <1>1, <2>1, <2>2, <2>3, <2>4, <2>5; algebra

<1>3. ⟦d⟧ = D iff covers(d). Coverage is a property of resolved
      identities and ranges, never of entry names.
      PROOF:
  <2>1. If covers(d), then ⟦d⟧ = D.
        BY <1>1, <1>2; algebra
  <2>2. If some γ ∈ comp(R) is not in img(res), then
        ⟦d⟧ ⊆ {x : x(γ) = z_γ}, a strict subset of D because w_γ ≥ 1.
        BY <1>1, <1>2.4, <1>2.6; algebra
  <2>3. If some entry has lo > 0 or hi < 2^w − 1, then ⟦d⟧ ⊊ D.
        BY <1>2.6; algebra
  <2>4. A `borrowed` entry resolves to a borrowed identity, which differs
        from every interface identity; so it never puts an interface
        component into img(res), whatever it is called.
        BY <1>1.6, <1>2.3
  <2>5. Two domains with the same resolved entries and ranges have the same
        ⟦d⟧ whatever their names. In particular, renaming a `borrowed` entry
        never changes ⟦d⟧ or covers(d).
        BY <1>2.6, <2>4
  <2>6. QED
        BY <2>1, <2>2, <2>3, <2>4, <2>5

<1>4. For every x ∈ ⟦d⟧ that a conformant checker replays for an
      exhaustive claim, it has compared tup_R(x) with tup_Q(x) (resp. the row
      of T for x).
      PROOF:
  <2>1. The obligation of an `exhaustive` claim is: "re-execute every
        domain element on the reference interpreter / expected program;
        compare full tuples".
        BY ISA-12.1-P1.S1
  <2>2. Reading adopted: the full tuple of x is the final value of every
        component of comp(R), in comp(R) order. This includes the borrowed
        bundles, whose final value must equal their initial value.
        ASSUMED: §12.1 does not define the tuple or its encoding
        (interpreter SPEC-QUESTIONS 21).
  <2>3. Reading adopted: `expected: {program: h_Q}` names a region with the
        same interface slots (role, width, class) and the same borrow
        requirement, and tup_Q(x) is read slot by slot: (g, j) of R against
        (g, j) of Q, borrowed k against borrowed k, never by local name.
        ASSUMED: §12.1 says neither that Q's signature equals R's nor how
        their components correspond; the positional reading follows the ABI
        (ISA-9.4-P2.S1).
  <2>4. Reading adopted: row k of `table` is the expected tuple of the k-th
        element in the enumeration order of ISA-12.2-P1.S1, and a table with
        a row count other than |⟦d⟧| is ill-formed.
        ASSUMED: §12.1 fixes no table encoding or correspondence.
  <2>5. Each x ∈ D has one final basis tuple: an oracle is exact-permutation
        and an involution on (ins, tgt, fr), and its contract fixes every
        other component; a permutation action's unitary is a permutation
        matrix.
        BY ISA-5.2-P1.S1, ISA-5.2-P2.S1, ISA-5.2-P2.S3, ISA-5.3-P1.S4,
           ISA-14.1-P1.S1
  <2>6. Reading adopted: an oracle claim is replayed at one fixed budget B
        (F_B depends on B), namely B of the root schedule bound by the
        certificate.
        ASSUMED: an exhaustive claim has no `subject`, but ISA-12.3-P2.S3
        says that `exhaustive` claims with `expected: "oracle"` "bind to
        `schedule_hash` through their subject"; for `program` and `table`
        expectations no budget is named at all (ISA-5.2-P2.S2 makes the
        tuple depend on it).
  <2>7. QED
        BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6

<1>5. (A) If a conformant checker returns `Proof` for c, then c is
      exhaustive, covers(d), and tup_R(x) = tup_Q(x) for every x ∈ D.
      PROOF:
  <2>1. Reading adopted: "A `proof` label" in §12.2 is the checker's
        verdict on the claim. A producer-written `label` field, if present,
        is a request that the checker validates, never a premise.
        ASSUMED: the §12.1 payloads contain no `label` field, and §12.2 does
        not say who assigns the label. ISA-1.2-P4.S1 ("validates
        certificates (§12) without trusting producers") rules out taking a
        producer's label on trust.
  <2>2. `Proof` ⇒ ⟦d⟧ = D ⇒ covers(d).
        BY <2>1, ISA-12.2-P1.S2, <1>3
  <2>3. `Proof` ⇒ c is exhaustive: a `sampled` claim is "replay;
        **evidence**".
        BY ISA-12.1-P2.S1
  <2>4. `Proof` ⇒ every element of ⟦d⟧ = D was replayed and its tuples were
        equal. A mismatch is a failed obligation, so the outcome is `Reject`,
        not `Proof`.
        BY <2>2, <2>3, <1>4
  <2>5. A classifier that decides coverage from names, i.e. N(d) ⟺ every
        interface name of R occurs among the names of all entries of d and
        all ranges are full, is unsound: N(d) holds while ⟦d⟧ ≠ D, and a
        checker using it issues a false `Proof`.
        PROOF:
    <3>1. Let R = `action @a (t:wire mut, u:wire mut)` with body
          `b = qborrow any 1; cx t u; qreturn b`, and Q = `@identity` with
          the same signature and body `b = qborrow any 1; x u; x u;
          qreturn b`. comp(R) = {(mut,0)=t, (mut,1)=u, (borrowed,0)=b},
          |D| = 8.
          BY <1>1, ISA-4.3-P2.S1
    <3>2. Let d = {bundles: [{u,1,0,1}], borrowed: [{t,1,0,1}],
          regions: []}. The names {u, t} include every interface name and
          every range is full, so N(d) holds.
          BY ISA-12.2-P1.S1; algebra
    <3>3. res(u-entry) = (mut,1) and res(t-entry) = (borrowed,0), so
          (mut,0) ∉ img(res), ⟦d⟧ = {x : x(t) = z} has 4 elements and
          ⟦d⟧ ≠ D.
          BY <1>2.3, <1>3.2, <1>3.4
    <3>4. On ⟦d⟧ with z = 0, `cx t u` is the identity, so all 4 compared
          tuples agree. At (t,u,b) = (1,0,0), R gives (1,1,0) and Q gives
          (1,0,0).
          BY ISA-4.4-P1.S1, ISA-4.4-P2.S1; algebra
    <3>5. QED: a checker that uses N replays the 4 elements, finds no
          mismatch and returns `Proof`, which contradicts <2>2 and the
          conclusion of (A).
          BY <3>2, <3>3, <3>4, <2>2
  <2>6. For a permutation-valued R, `Proof` against a permutation P (table
        or Q) supplies the premise of T3, "the final basis tuple equals P(x)
        for every x ∈ D", hence V|span(D) = P. This is the premise that
        INV-3 1.5.2 assumes for borrow frames ("established by a T3 claim
        over the frame's full domain").
        BY <2>4, ISA-14.3-P3.S1, ISA-5.6-P2.S2
  <2>7. QED
        BY <2>2, <2>3, <2>4, <1>3.4

<1>6. (B) If ¬covers(d) or c is sampled, the outcome is at most
      `Evidence(n)` with the exact n, and it depends only on the replayed
      elements.
      PROOF:
  <2>1. ¬covers(d) ⇒ ⟦d⟧ ≠ D ⇒ the claim is "`evidence` with the covered
        fraction", never `Proof`.
        BY <1>3, ISA-12.2-P1.S2, <1>5.1
  <2>2. A sampled claim is evidence.
        BY ISA-12.1-P2.S1
  <2>3. For exhaustive c, the replayed elements are exactly ⟦d⟧, so
        n = |⟦d⟧| = Π_e (hi_e − lo_e + 1), where a `cells: "all"` entry
        contributes 2^{W·2^A}. Since ⟦d⟧ ⊆ D, the covered fraction n / |D|
        is at most 1. It is less than 1 iff ¬covers(d).
        BY <1>2.6, <1>1.7, <1>3; algebra
  <2>4. A sampled claim's draws are elements of ⟦d⟧: each component of the
        domain is drawn in [lo, hi], "components are consumed in enumeration
        order", and omitted components keep z. So n = `count` replayed
        draws, of which at most `count` are distinct.
        BY ISA-12.1-P8.S1, ISA-12.1-P2.S1, <1>2.4
  <2>5. Reading adopted: a sampled claim reports n = `count` (draws
        replayed). If a fraction is reported, it is (distinct draws) / |D|.
        ASSUMED: §12.2 asks for "the covered fraction", which is undefined
        for draws with repetition, and §12.1 says only "replay; evidence".
  <2>6. The outcome is a function of tup_R and tup_Q (or T) restricted to
        the replayed elements. The checker executes nothing else.
        BY ISA-12.1-P1.S1, ISA-12.1-P2.S1, <1>4
  <2>7. So `Evidence` licenses no conclusion about D ∖ ⟦d⟧ beyond what
        agreement on ⟦d⟧ implies by itself. Instance: the pair of 1.5.5
        agrees on its 4 elements and differs at (1,0,0).
        BY <2>6, <1>5.5
  <2>8. The literal clause "never implies agreement outside the enumerated
        elements" is false in one corner. Let |D ∖ ⟦d⟧| = 1, say
        D ∖ ⟦d⟧ = {x₀}, and let R and Q be bijections of D: an oracle (an
        involution with its other components fixed) or a permutation action
        against a permutation Q. Agreement on ⟦d⟧ forces
        tup_R(x₀) = tup_Q(x₀), the only value left. The label is still
        `Evidence`.
        OPEN (refuted): limits the theorem, not the spec. With
        |D ∖ ⟦d⟧| ≥ 2, or with a table, agreement outside ⟦d⟧ is not
        forced in general (<2>7).
  <2>9. QED
        BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6, <2>7

<1>7. (C) A checker that cannot materialise or enumerate a legal domain
      reports a host limitation.
      PROOF:
  <2>1. The domain schema has no cap on `width`, on A of a region[A,W], on
        the length of a cell list or on `count`. So a claim over a
        region[63,1] with `cells: "all"` and `count` 1 is legal.
        BY ISA-12.2-P1.S1, ISA-12.1-P2.S1, ISA-3.3-P1.S1
  <2>2. Hence `Reject` on a legality ground (an "invalid domain", an
        out-of-range region, a width cap) is false for such a claim.
        BY <2>1, <1>2.5
  <2>3. Neither label may be issued for elements that were not replayed:
        `Proof` needs every element of D (<1>5.4), and `Evidence(n)` counts
        replayed elements (<1>6.3, <1>6.4).
        BY <1>5.4, <1>6.3, <1>6.4, ISA-12.1-P1.S1
  <2>4. A single element of a domain over region[A,W] carries 2^A·W bits,
        and |D| can reach 2^{W·2^A}. So whether a legal claim can be checked
        depends on the host, even at `count` 1.
        BY ISA-3.3-P14.S1, <1>1.7; algebra
  <2>5. 0.8 names no checker outcome for "legal, but not checkable on this
        host". The outcomes it names are `proof` and `evidence`
        (ISA-12.2-P1.S2) and rejection ("too few witnesses: reject",
        ISA-12.1-P5.S1; "an inconclusive test rejects", ISA-12.1-P7.S5).
        OPEN: the missing outcome. By analogy with ISA-12.1-P7.S5, an
        inconclusive check rejects the claim, which is honest only if a
        reject never reads as "the certificate is ill-formed" (<2>2).
  <2>6. Reading adopted: such a checker returns `HostLimit(§12.2)`, which
        is distinct from `Reject` and from both labels, names the quantity
        that does not fit, and terminates normally (no abort, no panic). The
        claim then counts as unverified for T3, §5.6 and §12.3.
        ASSUMED: needs <2>5's sentence; the repository rule "fail fast,
        fail loud" and the interpreter's R21 design take this reading.
  <2>7. Under <2>6, a host-limited claim supplies no T3 premise (1.5.6)
        and no evidence count.
        BY <2>3, <2>6, <1>5.6
  <2>8. QED
        BY <2>2, <2>3, <2>6, <2>7

<1>8. QED: (A) by <1>5; (B) by <1>6, limited by <1>6.8; (C) by <1>7 under
      reading <1>7.6.
      BY <1>5, <1>6, <1>7
```

## Witnesses

Interpreter tests are in `bennett-isa-spec-review/reference-interpreter` at
`9851a9c` (the round-22 snapshot). They are evidence, not justification.

| Witness | Steps |
|---|---|
| `tests/r1_certificates.rs::sol_r4_b03_expected_program_uses_signature_positions_for_renamed_bundles` (renamed and reordered bundles: `Proof`; one bundle omitted: `Evidence { elements: 2 }`; borrowed entries `z`, `b` do not affect coverage) | 1.2.2, 1.2.4, 1.3.2, 1.4.3, 1.6.3 |
| `::sol_r4_b03_expected_program_maps_renamed_region_cells_positionally` (`cells: "all"`: `Proof`; one listed cell: `Evidence { elements: 8 }`) | 1.2.2, 1.3.3, 1.6.3 |
| `::sol_r4_b03_positions_distinguish_same_width_slots_and_crossed_names` (crossed local names; wrong slot fails §12.1) | 1.1.4, 1.4.3 |
| `::b14_checked_borrow_claim_and_altered_or_incomplete_claims` (full domain with borrowed `b`: `Proof`; one wrong table row: §12.1; t up to 7, or no borrowed entry: not a §5.6 proof) | 1.3.3, 1.4.4, 1.5.4, 1.5.6, 1.6.1 |
| `::r17_sample_count_default_replays_and_present_values_validate` (sampled: `Evidence` with the replayed count) | 1.6.2, 1.6.4, 1.6.5 |
| `tests/r21_resources.rs::r21_b01_claims_public_consumers` → `r21_resource_child` family `claims` (all-cells and explicit-list region domains at A ∈ {0,1,3,62,63,64,127,128} through `verify_claim` and `verify_certificate` under RLIMIT_AS; A < 4 give `Evidence { elements: 1 }`, larger A a §12.2 host-limit error, no abort; the small all-cells exhaustive control is `Proof`) | 1.7.1, 1.7.2, 1.7.4, 1.7.6 |
| `tests/r21_resources.rs::r21_independent_witness_pins` (the reviewer objects, including `sol-claim-region-all-sampled.bisa`, keep their hashes) | 1.7.1 |
| `tests/r21_resources.rs::r21_cli_cardinality_and_sparse_output` (huge sample counts are entered lazily and stop at a reported storage error) | 1.7.3, 1.7.6 |
| Council files `bennett-isa-spec-review/.tmp/r22-sol/false-cnot-proof.bisa` (`Proof` at `9851a9c`, should be `Evidence { elements: 4 }`), `false-cnot-renamed-control.bisa` (borrowed entry renamed `b`: `Evidence { elements: 4 }`), `false-cnot-full-control.bisa` (t added: §12.1 mismatch at element 4) | 1.3.5, 1.5.5 (all of it), 1.6.3, 1.6.7; counter-witness until an R22 test lands |
| Council file `.tmp/r22-sol/domain-role-spoof.bisa` (`action @a (t:wire mut)`, body `x t`, domain only `borrowed: [t]`, table `[[1,0],[1,1]]`; no verdict recorded in the packet) | 1.5.5 with a table instead of Q (the row for t = 1 is never checked) |

No test exists yet for 1.2.5 (duplicate entries, width mismatch), 1.4.6
(the budget of an oracle claim with a `program` expectation) or 1.6.8. The
interpreter's `Object::domain` at `9851a9c` (`src/claims.rs`, lines 251 to
336) is the classifier N of 1.5.5: it puts the names of `bundles` and
`borrowed` entries into one set and tests the interface names against it.
It also sets `full = false` for every explicit cell list, so a list that
names every cell with full ranges gets `Evidence`. That is conservative,
and it agrees with (A), but under 1.3.1 such a list covers D.

## Council blockers as instances

- **R21-B01** (SOL-R21-B01, CODEX-R21-B01; a legal region[63,1]
  `cells: "all"` sampled claim with `count` 1 aborted the process with
  SIGABRT while the domain components were being expanded): violates
  1.7.6 (no normal termination and no `HostLimit` outcome). The council's fix
  rules out the two dishonest alternatives as well: capping A or relabelling
  the range as invalid violates 1.7.2, and labelling the claim violates
  1.7.3. Its root cause is 1.7.4: a `count` of 1 does not bound the size
  of an element. 0.8 gives the checker no outcome to report (1.7.5), so the
  determining text the council cited (§12.2 schema, §4.6 "2^A cells") fixes
  legality, not the outcome.
- **SOL-R22-B01** (borrowed entry spelled `t` discharged the interface
  bundle `t`, which was zero-filled; CNOT certified equal to identity):
  violates 1.3.4, and through it 1.5.2 and the conclusion of (A). The
  classifier was N of 1.5.5, which checks names where 1.3 requires resolved
  identities. The zero fill (1.2.4) is not itself the defect: it is
  harmless once ⟦d⟧ ≠ D is recognised, and the verdict then becomes
  `Evidence(4)` by 1.6.3. The renaming control is an instance of 1.3.5:
  renaming a borrowed entry must not change the verdict. The council said
  no spec edit is required. Under readings 1.2.2 and 1.2.3 that holds, but
  0.8 states neither reading.

## Spec gaps

1. (1.1.3, next to `ISA-5.3-P1` (`ISA-5.3-P1.S1`)) ASSUMED. Needed sentence:
   "**Domain D** of an action: every value of each `mut` and `in` bundle, of
   every cell of every `mut` and `in` region and of every borrowed bundle;
   workspace at 0, local regions at image." Also make the factor list read
   "mut ⊗ ins ⊗ mem ⊗ (borrowed)", or say why `in` regions are absent.
2. (1.2.2, next to `ISA-12.2-P1` (`ISA-12.2-P1.S1`)) ASSUMED. Needed
   sentence: "The `name` of a `bundles` or `regions` entry is the local name
   of an interface bundle or region in the subject's header, and the entry
   denotes that interface slot (role and position); a name that is not one
   is an error. Coverage is decided on the denoted slots and cells, never on
   names."
3. (1.2.3, next to `ISA-12.2-P1` (`ISA-12.2-P1.S1`)) ASSUMED. Needed
   sentence: "The k-th `borrowed` entry denotes the subject's k-th borrowed
   bundle in the order of its borrow requirement; its `name` is a label
   only, may coincide with any other name, and never denotes an interface
   slot." This answers whether a borrowed name may coincide with an
   interface name: in 0.8 nothing forbids it and nothing gives it meaning.
4. (1.2.4, next to `ISA-12.2-P1` (`ISA-12.2-P1.S2`)) ASSUMED. Needed
   sentence: "A component of D that the domain omits is held at 0 on every
   element; the domain then denotes the slice of D at 0, and its covered
   fraction is the slice's size over |D|." The alternative is to forbid
   omission ("every component of D appears in the domain") and to express
   partial domains only by ranges.
5. (1.2.5, next to `ISA-12.2-P1` (`ISA-12.2-P1.S1`)) ASSUMED. Needed
   sentence: "Each component appears at most once; `width` equals the
   component's width; 0 ≤ lo ≤ hi < 2^width; cell indices are below 2^A;
   otherwise the claim is ill-formed."
6. (1.4.2, next to `ISA-12.1-P1` (`ISA-12.1-P1.S1`)) ASSUMED. Needed
   sentence: "The full tuple of a domain element is the final value of every
   component of D, in the order: interface bundles by role and slot,
   interface region cells, borrowed bundles." Also fix the table row
   encoding (gap 8).
7. (1.4.3, next to `ISA-12.1-P1` (`ISA-12.1-P1.S1`)) ASSUMED. Needed
   sentence: "An expected program has the subject's interface slots and
   borrow requirement; its components correspond to the subject's by role and
   position, not by name."
8. (1.4.4, next to `ISA-12.1-P1` (`ISA-12.1-P1.S1`)) ASSUMED. Needed
   sentence: "`table` holds one full tuple per domain element, in
   enumeration order; each entry is a §9.1 integer; a table of any other
   length is an error."
9. (1.4.6, next to `ISA-12.3-P2` (`ISA-12.3-P2.S3`) and `ISA-12.1-P1`
   (`ISA-12.1-P1.S1`)) ASSUMED. ISA-12.3-P2.S3 binds an `exhaustive` claim
   "through [its] subject", but the exhaustive payload has no subject.
   Needed: a `subject` (or at least `schedule_hash` and `budget`) in the
   `exhaustive` and `sampled` payloads of oracle claims, and the sentence
   "an oracle claim is replayed at the budget of its subject".
10. (1.5.1, next to `ISA-12.2-P1` (`ISA-12.2-P1.S2`)) ASSUMED. Needed
    sentence: "The label (`proof` or `evidence`) is the checker's verdict; a
    certificate does not assert it, and a written `label` field is
    rejected (or: is checked and rejected if it claims more than the
    verdict)."
11. (1.6.5, next to `ISA-12.1-P2` (`ISA-12.1-P2.S1`) and `ISA-12.2-P1.S2`)
    ASSUMED. Needed sentence: "A sampled claim reports `count` replayed
    draws; its covered fraction is the number of distinct elements drawn over
    |D|."
12. (1.7.5, next to `ISA-12.2-P1` (`ISA-12.2-P1.S1`,
    `ISA-12.2-P1.S2`)) OPEN; reading 1.7.6 ASSUMED. Needed sentence: "A
    checker that cannot replay a legal claim on its host reports the claim
    as **unchecked (host limit)**. This is neither a label nor a rejection of
    the certificate, and an unchecked claim discharges no obligation of §5.6,
    T3 or §12.3." State also that 0.8 imposes no cap on domain widths, A or
    `count`, so that no checker may invent one.

1.5.5 and 1.6.8 need no spec text: 1.5.5 refutes a classifier, and 1.6.8
limits the theorem's wording.
