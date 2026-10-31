# INV-5: schedule selection is sound, admission is exact, defaults are values

Spec: ISA 0.8 (`clauses/ISA-0.8.json`). Format: `proofs/FORMAT.md`.

## Informal statement

A schedule (§11) is a list of entries. Each entry selects generated operations
by `path`, `step`, `phase`, `iteration` and `generated`, and makes one decision of
one `kind`. Among the entries that match an operation, the most specific one
applies. Two maximal matches make the schedule invalid, and document order
never matters. Fields the schedule leaves out take their §11 / §17 defaults.
For a consumer this gives three properties:

- (A) **Selection soundness.** In an admitted schedule, each concrete operation
  has at most one maximal matching entry per kind. Here the `step` of an
  operation inside a nested oracle invocation ranges over that invocation's
  own budget, not the enclosing one.
- (B) **Admission exactness.** Admission rejects a schedule (on matching
  grounds) iff some operation that is generated under the declared budgets
  has two or more maximal matching entries of one kind. So shadowed ties are
  admitted, genuine ties are rejected, and document order plays no part.
- (C) **Defaults.** An omitted field with a §17 default is matched and resolved
  exactly as if the default had been written.

Council rounds 15 to 18 on the reference interpreter each found a violation of
one of these properties (see "Council blockers as instances"). The proof shows
which parts hold in 0.8, under which readings, and where the text is silent or
false:

- (A) and (B) hold once three readings are fixed: what a `lift.call` contributes
  to a chain, which invocation level a `step` reads, and what "incomparable"
  means. They also need the consumer to know the nested token budgets.
- (B) is false for any admission that sees only the object and the schedule
  (`params_hash`). Such an admission cannot know the nested step domains.
- (C) holds for `expansion` (except that the hash of `ref` is unstated) and for
  `sampler.count`. It is false as written for `epsilon_share`, whose default is
  in general no finite decimal. It is also false for the defaults of uncovered
  operations, which the selector language cannot always write down.

## Precise statement

Fix a well-formed object O, a schedule S with `bindings.region` = R,
`bindings.budget` = [B_0, heap, stack, depth] (`ISA-11-P3.S2`), and the
classical arguments of the run, of which S records only `params_hash`
(`ISA-9.2-P2.S13`). Let β be the map from each token argument to the budget it
was issued with (`ISA-10-P7.S2`, `bennett_budget`).

DEFINE an **invocation** as the root invocation of R, or an invocation created
by a region call (`oracle_call`, `action_call`, `ISA-4.7-P1.S1`) or a
`lift.oracle_call` (`ISA-4.2-P21.S1`) inside another invocation. An **oracle
invocation** ι runs B_ι forward steps and B_ι inverse steps
(`ISA-5.2-P3.S1.2`, `ISA-5.2-P3.S1.4`). B_root = B_0. For a nested oracle
invocation, B_ι = β(tk), where tk is the call's token.

DEFINE a **concrete operation** o = (c, τ, φ, g):
- c = (e_1, …, e_m) is its invocation chain (`ISA-11-P4.S1`). Each
  e_j = (sym_j, pt_j, it_j), where it_j is the vector of 0-based qloop
  repetition counts at pt_j (`ISA-11-P4.S2`).
- τ maps each oracle invocation level ℓ on the chain to the forward machine
  step τ(ℓ) ∈ 1..B_ℓ during which o is generated.
- φ maps the same levels to `fwd` or `rev`.
- g is o's ordinal within the expansion of the row at pt_m in the bound
  library (`ISA-11-P4.S3`).

Ω(O, S, β) is the set of concrete operations of the root invocation.

DEFINE an **entry** e = (P, s, h, γ, k, payload): P is its `path`, s its
`step`, h its `phase`, γ its `generated` and k its `kind`. Let lv(P) be the
invocation level whose step and phase the entry reads (step 1.1.8). Then
`match(e, o)` holds iff all four conditions hold (`ISA-11-P4.S2`,
`ISA-11-P4.S3`, `ISA-11-P4.S4`):
- P is an element-wise prefix of c;
- s = `*` or s = τ(lv(P));
- h = `*` or h = φ(lv(P));
- γ = `*` or γ = g.

DEFINE κ(e) ∈ ℕ^5 as the specificity tuple, ordered lexicographically.
DEFINE M_k(o) = {e ∈ S of kind k : match(e, o)} and Max_k(o) = {e ∈ M_k(o) :
κ(e) = max κ[M_k(o)]}. Entries are counted by array position.
DEFINE `Tie(S, β)` ⟺ ∃ o ∈ Ω(O, S, β), ∃ k : |Max_k(o)| ≥ 2.

THEOREM INV-5 (as intended). For every well-formed O and every S that passes
every non-matching admission check:
- (A) if S is admitted, then |Max_k(o)| ≤ 1 for all o ∈ Ω(O, S, β) and all k.
  The step coordinate τ(ℓ) of o ranges over 1..B_ℓ for its own level ℓ.
- (B) admission rejects S iff Tie(S, β). The decision depends only on the
  multiset of entries.
- (C) for every field f with a §17 default d_f, the schedule S_f that omits f
  and the schedule S[f := d_f] that writes it have the same Ω, the same
  `match` and the same resolved decision for every o and k.

THEOREM INV-5' (what holds in 0.8).
- (A) and (B) hold for **bound admission**, where the consumer knows β. They
  need the readings 1.1.4 (lift.call contributes no chain element), 1.1.7
  (step and phase on action-level operations), 1.1.8 (step level) and 1.2.2
  (equal maximal tuple), and they cover only operations generated by rows
  (1.1.6). Cleanup-kind entries are also open (1.2.4).
- (B) fails for **unbound admission**, which sees only O and S (1.4.3).
- (C) holds for `bindings.expansion` (up to the unstated hash of `ref`, 1.6.3.3)
  and for `sampler.count`. It is refuted for `epsilon_share` taken as a written
  decimal (1.6.4.3) and for uncovered-operation defaults taken as written
  entries (1.6.7).

## Hypotheses

ISA-3.2-P10.S1, ISA-3.6-P1.S1, ISA-3.6-P4.S1, ISA-3.6-P6.S1, ISA-3.6-P11.S1,
ISA-3.7-P20.S2, ISA-4.2-P1.S2, ISA-4.2-P21.S1, ISA-4.6-P8.S1, ISA-4.6-P9.S1,
ISA-4.7-P1.S1, ISA-5.1-P1.S7, ISA-5.2-P1.S1, ISA-5.2-P2.S2, ISA-5.2-P3.S1.2,
ISA-5.2-P3.S1.4, ISA-5.4-P1.S3, ISA-8.1-P1.S1, ISA-8.1-P10.S1, ISA-8.1-P10.S2,
ISA-9.2-P2.S13, ISA-10-P1.S4, ISA-10-P4.S1, ISA-10-P7.S2, ISA-11-P2.S1,
ISA-11-P2.S2, ISA-11-P2.S3, ISA-11-P3.S2, ISA-11-P4.S1, ISA-11-P4.S2,
ISA-11-P4.S3, ISA-11-P4.S4, ISA-11-P4.S5, ISA-11-P4.S6,
ISA-11-P4.S9, ISA-12.1-P2.S1, ISA-17-P7.S2, ISA-17-P8.S2, ISA-17-P10.S1,
ISA-17-P10.S2, ISA-17-P11.S1, ISA-17-P14.S2.

## Proof

```
<1>1. Ω(O, S, β) is a finite set determined by O, the bindings of S and β,
      and match(e, o) is a function of (e, o).
      PROOF:
  <2>1. Every generated operation has an invocation chain, outermost first,
        "ending in its own point"; `path` matches iff it is an element-wise
        prefix, `iteration` being `*` or the exact repetition vector.
        BY ISA-11-P4.S1, ISA-11-P4.S2
  <2>2. A `lift.call` creates no new step machine: lproc rows are rows of the
        calling oracle's index space I, every forward step runs every row of
        I under its dispatch predicate, and "callee rows are charged to the
        caller's budget".
        BY ISA-3.6-P1.S1, ISA-3.6-P6.S1, ISA-3.7-P20.S2, ISA-4.2-P1.S2
  <2>3. The literal chain is not a function of the generated operation for
        rows reached through `lift.call`. Counterexample: oracle @o has
        `lift.call @f` rows at p1 and p2 on the two arms of an input branch,
        and @f has a gate-generating row q. At step t the circuit holds one
        expansion of q under at_q (<2>2). For inputs routed through p1 its
        chain is [(o,p1),(f,q)]; for inputs routed through p2 it is
        [(o,p2),(f,q)]. Under recursion the chain also grows with the dynamic
        depth. Two equal-specificity gate entries on [(o,p1),(f,q)] and
        [(o,p2),(f,q)] that choose different realisations each match one
        "chain" of the same gate. No per-operation choice satisfies both.
        OPEN (refuted): `ISA-11-P4.S1` puts "the `lift.call`s that led to it"
        into the chain of an operation the step machine generates
        independently of any call.
  <2>4. Reading adopted: the chain contains the points of region calls and
        `lift.oracle_call`s, and then the operation's own row point. A row of
        an lproc in an oracle's closure is one generated operation per step,
        whichever `lift.call` led to it.
        ASSUMED: the only reading under which the chain is static (<2>3); the
        interpreter's chain domains behave this way (README "Design (schedule
        matching)").
  <2>5. Each oracle invocation ι has a step domain 1..B_ι of its own:
        B_root = B_0, and a nested invocation's budget is that of its token.
        PROOF:
    <3>1. The root runs exactly B_0 forward steps (t = 1..B_0) and the inverse
          steps t = B_0..1, with B_0 = `bindings.budget[0]`.
          BY ISA-11-P3.S2, ISA-5.4-P1.S3, ISA-3.6-P4.S1, ISA-5.2-P3.S1.2,
             ISA-5.2-P3.S1.4, ISA-3.6-P11.S1
    <3>2. `lift.oracle_call @o tk …` applies the callee contract F_B. B is the
          budget of the token tk, which `bennett_budget(program, region,
          steps, …)` issued for that region. Tokens are program-scoped
          classical values.
          BY ISA-4.2-P21.S1, ISA-5.2-P2.S2, ISA-10-P7.S2, ISA-10-P1.S4,
             ISA-3.2-P10.S1
    <3>3. `step` and `phase` are "per invocation level, so a nested oracle's
          step is selected by a path element for the nested call". The step
          coordinate of an operation at nested level ℓ therefore ranges over
          1..B_ℓ = 1..β(tk), and B_ℓ is not bounded by B_0. Example (council
          R18): B_0 = 2, β(tk) = 3. The nested `lift.retval` row of @inc8
          runs at nested step 3, which exists although 3 > B_0.
          BY ISA-11-P4.S3, <3>1, <3>2
    <3>4. QED
          BY <3>1, <3>2, <3>3
  <2>6. Operations generated by a step but by no row have no "own point" and
        no row ordinal: dispatch comparators, successor XORs, commit of pc,
        clean dispatch. Construction allocation, commit and release also lie
        outside every step t ∈ 1..B.
        OPEN: `ISA-11-P2.S2` lists them as generated operations, but
        `ISA-11-P4.S1` and `ISA-11-P4.S3` define the chain end and
        `generated` only for a row's expansion. No entry can select them
        with certainty, so it is unknown which defaults or entries govern
        them.
  <2>7. An operation at action level (no enclosing oracle invocation) has
        no step and no phase. Only `step: "*"` and `phase: "*"` match it.
        ASSUMED: `ISA-11-P4.S3` defines `step` only "of an oracle-generated
        operation" and does not say how a concrete `step`/`phase` treats
        other operations. The `adj` of an action call is not a `rev` phase.
  <2>8. Reading adopted for the level an entry's `step` and `phase` read:
        lv(P) is the oracle invocation entered by the last oracle-call
        element of P, or the root if P has none. A nested step is therefore
        selected exactly when the path contains the nested call. Two other
        readings fit the text: (R1) the level that owns P's last point,
        which differs only when P ends at a call point; and (R2) the
        operation's own innermost level, under which `path: []`,
        `step: 3` selects nested step 3 everywhere. All three agree on the
        R18 witness.
        ASSUMED: `ISA-11-P4.S3` says "per invocation level" but gives an
        entry one `step` and does not say which level it reads.
  <2>9. Ω is finite: oracle and action call graphs are acyclic (recursion
        only through `lift.call`, which adds no element by <2>4), qloop trip
        counts are literals, every level has finitely many steps (<2>5), and
        each row has finitely many ordinals in the bound library.
        BY ISA-5.1-P1.S7, ISA-4.6-P8.S1, ISA-4.6-P9.S1, ISA-11-P2.S3, <2>4,
           <2>5
  <2>10. QED
        BY <2>1, <2>4, <2>5, <2>8, <2>9
<1>2. For each kind, the maximal set Max_k(o) is well defined and depends
      only on the multiset of entries.
      PROOF:
  <2>1. Specificity is "the lexicographic tuple (path length, number of
        non-`*` iterations, step non-`*`, phase non-`*`, generated non-`*`)".
        Lexicographic order on ℕ^5 is total, so the maximal tuple of a
        nonempty finite M_k(o) exists and is unique.
        BY ISA-11-P4.S4; algebra
  <2>2. "Two incomparable maximal matching entries" means two distinct
        entries (array positions) with equal maximal tuple. Exact duplicates
        count as such a pair.
        ASSUMED: under the stated total order no two distinct tuples are
        incomparable, so the word "incomparable" is either void or alludes to
        the product order. Under the product order, (2,0,0,0,0) and
        (1,1,1,1,1) would both be maximal and invalidate the schedule, while
        the lexicographic order selects the first.
  <2>3. "Selectors are resolved independently per `kind`": entries of
        different kinds never tie with, nor shadow, each other.
        BY ISA-11-P4.S5
  <2>4. Which operations an entry of each kind can match: `gate` matches gate
        operations, `alloc` allocations, `start` every operation; `cleanup`
        is open.
        PROOF:
    <3>1. `gate`, `alloc`, `start`: a kind is matched only against
          operations for which it makes a decision.
          ASSUMED: `ISA-11-P4.S4` matches by selectors alone and never
          relates `kind` to the operation's class (`ISA-11-P2.S1`: gates,
          allocations, releases).
    <3>2. `cleanup`: a cleanup is a sequence of operations with one
          realisation (`reverse` or `recompute`). Per-operation selection
          can give operations of one cleanup different realisations, and
          the spec names no operation that carries the cleanup's identity.
          OPEN: which generated operation(s) a `cleanup` entry is matched
          against.
    <3>3. QED
          BY <3>1, <3>2
  <2>5. Document order: M_k(o) and Max_k(o) are defined by membership and κ,
        so they are invariant under every permutation of `entries`; "document
        order is never significant".
        BY ISA-11-P4.S4; algebra
  <2>6. QED
        BY <2>1, <2>2, <2>3, <2>5
<1>3. S is invalid on matching grounds iff Tie(S, β). The quantifier ranges
      over Ω(O, S, β), the operations generated under the declared budgets,
      not over all budgets.
      PROOF:
  <2>1. The schedule is invalid iff some generated operation has two
        maximal matching entries of one kind.
        BY ISA-11-P4.S4, ISA-11-P4.S5, <1>2
  <2>2. Operations beyond a level's budget are not generated: "B is enforced
        by running exactly B steps". An entry whose selectors meet only such
        operations is dead and creates no tie.
        BY ISA-5.4-P1.S3, <1>1
  <2>3. QED
        BY <2>1, <2>2
<1>4. (B) Admission exactness.
      PROOF:
  <2>1. A consumer must reject an invalid schedule before executing it: it
        "validates every binding and every path against the object and
        executes verbatim or rejects".
        ASSUMED: `ISA-11-P4.S9` does not list the maximal-match condition
        among the checks it names, nor fix when it is checked. We read
        "executes verbatim" as excluding any execution that has already
        begun when a tie is first met.
  <2>2. Bound admission (β known) decides Tie(S, β) exactly.
        PROOF:
    <3>1. Shadowed ties are admitted. Let A and B tie at κ and let every
          o ∈ match(A) ∩ match(B) ∩ Ω be matched by some C_o of the same kind
          with κ(C_o) > κ (one C or a union). Then A, B ∉ Max_k(o) for every
          such o, so A and B contribute no tie.
          BY <1>2.1, <1>3; algebra
    <3>2. Genuine ties are rejected. If some o ∈ match(A) ∩ match(B) ∩ Ω has
          no matching entry of its kind above κ, then {A, B} ⊆ Max_k(o) and
          Tie(S, β) holds.
          BY <1>2.1, <1>2.2, <1>3
    <3>3. A pairwise criterion ("reject iff two equal-κ entries of one kind
          have overlapping selector boxes") rejects every shadowed tie in
          <3>1, so it is sound but not complete.
          BY <3>1; algebra
    <3>4. The step coordinate must use each level's own domain (<1>1.5.3).
          Using B_0 for a nested level ℓ removes the operations with
          B_0 < τ(ℓ) ≤ B_ℓ, so ties there are admitted. It adds dead
          operations with B_ℓ < τ(ℓ) ≤ B_0, so schedules valid for the actual
          budget are rejected.
          BY <1>1.5.3, <1>3; algebra
    <3>5. Since Ω is finite and match is a function (<1>1), enumerating
          Ω × S × S, or intersecting and subtracting the selector boxes over
          each level's domain, decides Tie(S, β). The result depends only on
          the multiset of entries.
          BY <1>1, <1>2.5, <1>3; algebra
    <3>6. QED
          BY <3>1, <3>2, <3>5
  <2>3. Unbound admission, which sees only O and S, cannot decide Tie. Ω
        depends on β through the nested step domains (<1>1.5.3). S records
        only `params_hash`, "SHA-256 of the JCS array of classical parameter
        values", and a token's interchange value is "the 8-byte little-endian
        id", which carries no budget. The R18 schedule (two equal gate
        entries on nested step 3) is invalid with β(tk) = 3 and valid with
        β(tk) = 2, under the same object and bindings. If a token's JCS value
        is its id, the `params_hash` is the same too. So every procedure that
        reads only (O, S) fails (B) for one of the two runs: root-B clipping
        admits the first (R18-B01), and a symbolic "some budget" check
        rejects the second.
        OPEN (refuted for unbound admission): `ISA-11-P4.S9` and
        `ISA-9.2-P2.S13` do not say whether the matching check may be
        deferred until β is known, nor what JCS value a `token` parameter
        has in `params_hash`. Uses `ISA-10-P4.S1`.
  <2>4. QED
        BY <2>1, <2>2 for bound admission; <2>3 refutes (B) for unbound
        admission.
<1>5. (A) Selection soundness, for bound admission.
      PROOF:
  <2>1. Admitted implies ¬Tie(S, β).
        BY <1>4.2
  <2>2. Hence for every o ∈ Ω and kind k, |Max_k(o)| ≤ 1. Every τ(ℓ) is
        taken over 1..B_ℓ of its own level.
        BY <2>1, <1>3, <1>1.5
  <2>3. The decision for (o, k) is "the unique maximal one" if M_k(o) ≠ ∅,
        and the §11 default for uncovered operations otherwise. Both are
        independent of document order.
        BY ISA-11-P4.S4, ISA-11-P4.S6, <1>2.5
  <2>4. QED
        BY <2>2, <2>3
<1>6. (C) Defaults.
      PROOF:
  <2>1. Matching reads only `path`, `step`, `phase`, `generated` and `kind`,
        and Ω reads only the bindings (region, budget, expansion) and β.
        Omitting a payload field (`realisation`, `epsilon_share`, `class`,
        `time`) therefore changes neither Ω, `match`, Max_k nor Tie.
        BY ISA-11-P4.S2, ISA-11-P4.S3, ISA-11-P4.S4, <1>1; algebra
  <2>2. Defaults act per entry: if the applying maximal entry omits a field
        with a default, the default applies. The field is not inherited from
        a less specific matching entry.
        ASSUMED: `ISA-11-P4.S4` says "the unique maximal one applies" and
        does not say whether fields merge across matching entries. Under
        field-wise merging, omission would not equal writing the default.
  <2>3. `bindings.expansion` omitted.
        PROOF:
    <3>1. "A schedule that names no `expansion` binds `ref` at the ISA's own
          version". The ordinals that `generated` selects, and with them Ω
          and `match`, are those of `ref`, exactly as for a written binding
          to `ref`.
          BY ISA-17-P8.S2, ISA-11-P2.S3, ISA-11-P4.S3, <2>1
    <3>2. "The ISA's own version" is the version in the object's header,
          not the consumer's.
          ASSUMED: `ISA-17-P8.S2` does not say whose version is meant.
    <3>3. The written equivalent `{ "name": "ref", "version": "0.8",
          "hash": H }` needs H, and the spec states no hash for `ref`.
          OPEN: `ISA-11-P2.S3`, `ISA-17-P8.S2` give no hash for the default
          binding, so "as if written" names an undetermined triple.
    <3>4. QED
          BY <3>1, <3>2 for matching and resolution; <3>3 for the written
          form.
  <2>4. `gate.epsilon_share` omitted.
        PROOF:
    <3>1. When a schedule assigns no share, eps = the region's `epsilon`
          divided equally among its ε > 0 operations.
          BY ISA-17-P10.S1, ISA-17-P10.S2, ISA-8.1-P10.S1
    <3>2. Whether an operation is "ε > 0" depends on the `gates` entry it
          matches, and that match depends on eps ("a `gates` entry whose
          `epsilon` … > `eps` does not match"). The count N in the default is
          therefore defined through the default itself.
          OPEN: `ISA-8.1-P10.S1`, `ISA-8.1-P10.S2`, `ISA-17-P10.S2` give no
          eps-independent definition of an ε > 0 operation, nor say which
          region's `epsilon` and which invocations' operations are meant
          when nested regions exist.
    <3>3. ε / N is in general no finite decimal (`epsilon` "0.1", N = 3
          gives 0.0333…). No written schedule carries it, so "exactly as if
          the default were written" is false. What holds is "as if the
          rational ε / N had been assigned".
          OPEN (refuted): `ISA-17-P10.S2` (with `ISA-11-P3.S2`, a share is a
          decimal) has no written equivalent.
    <3>4. An entry that sets `gate.realisation` and omits `epsilon_share`
          "assigns none", like an operation with no gate entry at all.
          BY ISA-17-P10.S1, <2>1, <2>2
    <3>5. QED
          BY <3>1, <3>4; <3>2 and <3>3 bound what "as if written" can mean.
  <2>5. A written decimal is used by its value: `epsilon` 0, "0", "0.0" and
        "0.000" all satisfy "exact-permutation only (ε = 0 everywhere)".
        Decimals are also compared and divided by value (<2>4.1).
        ASSUMED: §11 never says that a schedule decimal denotes the exact
        rational of its text. `ISA-8.1-P1.S1` says so only for targets
        ("numbers are exact rationals from their decimal text"). Uses
        `ISA-5.2-P1.S1`.
  <2>6. Uncovered operations (M_k(o) = ∅) resolve by rule: cleanup
        `reverse` (§17 item 6); gate `native` if a `gates` entry matches,
        else the first matching scheme; class = the first class in JCS
        order with capacity (§17 item 10); sequential timing. Each rule
        reads only o and the target, never another kind's entries or
        document order.
        BY ISA-11-P4.S6, ISA-17-P7.S2, ISA-17-P11.S1, <1>2.3, <1>2.5
  <2>7. An uncovered default cannot always be written as an entry without
        changing other operations. Take root R with a `lift.oracle_call` row
        r into @g, whose row q generates gate operations. Let E be a gate
        entry with `path` [] and `step` 1 (root level, <1>1.8) and scheme X.
        The gate o of q at root step 2 and nested step 1 is uncovered. Any
        entry that matches o has `path` at least [(R,r)] and can constrain
        only the nested step. It therefore also matches o', the same gate at
        root step 1, and outranks E there (path length is the first
        component of κ). Writing the default for o moves o' from X to
        `native`.
        OPEN (refuted): defaults of uncovered operations are not "as if
        written". This limits (C), not 0.8: `ISA-11-P4.S6` states them as
        rules. The cause is that an entry has one `step` for a chain with
        several levels (`ISA-11-P4.S3`).
  <2>8. `sampler.count` omitted (§17 item 13, the certificate analogue):
        2^16 = 65536 is an integer the claim can carry, so omission and the
        written value replay identically.
        BY ISA-17-P14.S2, ISA-12.1-P2.S1; algebra
  <2>9. QED
        BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6, <2>8; <2>4.3 and <2>7 refute
        the written form for item 9 and for uncovered operations.
<1>7. QED
      BY <1>5 for (A); <1>4 for (B) (bound admission; refuted unbound,
      1.4.3); <1>6 for (C) (refuted for item 9 as a written value, 1.6.4.3).
```

## Witnesses

Interpreter tests are in `bennett-isa-spec-review/reference-interpreter` at
`be9e6d1`. They are evidence, not justification.

| Witness | Steps |
|---|---|
| `tests/metadata.rs::schedule_requires_typed_bindings_selectors_and_unique_maxima`, `::schedule_paths_bind_real_program_points_and_loop_depths` | 1.1.1, 1.1.9, 1.3 |
| `tests/metadata.rs::r15_oracle_schedule_epsilon_uses_exact_decimal_value`; `tests/cli_xam.rs::r15_cli_oracle_schedule_epsilon_is_exact_by_value` (0, "0", "0.0", "0.000" admit; "0.1" rejects) | 1.6.5 |
| `tests/metadata.rs::r16_default_expansion_preserves_witness_and_identity`, `::r16_present_expansion_still_requires_closed_descriptor`, `::r16_schedule_consumer_resolves_default_and_preserves_explicit_binding`; `tests/cli_xam.rs::r16_cli_default_expansion_check_hash_preserves_pins`, `::r16_cli_malformed_present_expansion_rejects` | 1.6.3.1, 1.6.3.2 (resolves `ref` at the object's 0.8, supplies no hash: 1.6.3.3) |
| `tests/metadata.rs::r17_default_share_and_shadowed_witness_preserve_identity`, `::r17_present_gate_share_and_missing_realisation_reject`, `::r17_equal_share_consumer_keeps_exact_and_explicit_assignments`; `tests/cli_xam.rs::r17_cli_default_share_and_shadowed_tie_check_hash_pins`, `::r17_cli_malformed_gate_share_rejects` | 1.6.1, 1.6.4.1, 1.6.4.4 (the consumer keeps ε / N symbolic: consistent with 1.6.4.3) |
| `tests/metadata.rs::r17_shadowed_tie_all_orders_and_genuine_maximum_ties` (A/B/C in six orders; A/B alone and duplicate C reject) | 1.2.2 (duplicates), 1.2.5, 1.4.2.1, 1.4.2.2 |
| `tests/metadata.rs::r17_specificity_lattice_family_matches_brute_force` (1,200 schedules, 54 partitions each) | 1.2.1, 1.4.2.5 |
| `tests/metadata.rs::r17_union_coverage_partial_coverage_and_budget_domains`, `::r17_loop_iteration_union_and_empty_domains` | 1.4.2.1 (union cover), 1.3.2 (out-of-budget, zero-trip), 1.1.9 |
| `src/metadata_schema/tests.rs::r17_section17_defaults_are_optional_and_applied`; `tests/r1_certificates.rs::r17_sample_count_default_replays_and_present_values_validate` | 1.6.2, 1.6.8 |
| Council files `bennett-isa-spec-review/.tmp/r18-nested-budget-2.bisa` (root B = 2, token B = 3, two equal gate entries on nested step 3: admitted at `be9e6d1`, should reject) and `r18-nested-budget-3.bisa` (root B = 3 control: rejects) | 1.1.5.3, 1.4.2.4; counter-witness until an R18 test lands |

No test exists yet for 1.1.3 (two `lift.call` sites), 1.1.8 (the readings
differ on `path: []` with a concrete `step` over a nested operation), 1.4.3
(the same schedule under two token budgets) or 1.6.7. The interpreter's
`resolve_schedule` takes an operation with a single `step` and `phase`. It
cannot represent the per-level vector τ of a nested operation, so reading
1.1.8 is implicit in its domain code rather than in its interface.

## Council blockers as instances

- **R15-B02** (bindings `epsilon` "0.0" rejected as non-exact): violates
  1.6.5. Admission used the spelling instead of the value, so a written zero
  was not the value zero. The determining text is `ISA-5.2-P1.S1`. The
  value reading is ASSUMED because §11 has no decimal-value sentence.
- **R16-B01** (omitted `bindings.expansion` rejected): violates 1.6.3.1. The
  omitted field was treated as missing instead of as the default binding to
  `ref`.
- **R17-B01** (omitted `gate.epsilon_share` rejected in a realisation-only
  entry): violates 1.6.4.4, and through it 1.6.1. A payload omission changed
  the admission outcome.
- **R17-B02** (pairwise rejection of the shadowed A/B tie under C): violates
  1.4.2.1, and with it the "only if" half of (B). The implementation used
  the criterion of 1.4.2.3 instead of the per-operation quantifier of 1.3.
- **R18-B01** (nested step 3 > root B = 2, tie admitted): violates 1.1.5.3,
  and through 1.4.2.4 the "if" half of (B) and (A) (1.5.2). The step domain
  was taken from the root budget instead of the nested token. The fix the
  council asks for (a symbolic nested range when β is unknown) runs into
  1.4.3: without β no procedure is exact, and the symbolic one over-rejects.

## Spec gaps

1. (1.1.3 OPEN (refuted), 1.1.4 ASSUMED, next to `ISA-11-P4` (`ISA-11-P4.S1`)). Needed
   sentence: "The invocation chain contains the program points of region calls
   and `lift.oracle_call`s, then the operation's own point. `lift.call` adds
   no element: a row of an lproc in an oracle's closure generates one
   operation per step, whichever call led to it."
2. (1.1.5.2, next to `ISA-4.2-P21` (`ISA-4.2-P21.S1`) and `ISA-5.2-P2.S2`)
   implicit. The steps B of a `lift.oracle_call` callee are those of its
   token, as issued by `bennett_budget`. This is stated only through the C
   signature (`ISA-10-P7.S2`). Suggested sentence: "A nested oracle call runs
   its callee with the budget of its token."
3. (1.1.6, next to `ISA-11-P4` (`ISA-11-P4.S1`, `ISA-11-P4.S3`)) OPEN.
   Needed: a point, step and ordinal for every generated operation that no
   row generates. For example: "Dispatch, successor, pc-commit and
   clean-dispatch operations of step t carry the chain of their oracle
   invocation, step t and ordinals assigned by the expansion library under a
   pseudo-point `step`. Construction, commit and release carry the pseudo
   points `alloc`, `commit`, `release` and match only `step: "*"`."
4. (1.1.7, next to `ISA-11-P4` (`ISA-11-P4.S3`)) ASSUMED. Needed sentence: "An
   operation not generated by an oracle step matches only `step: "*"` and
   `phase: "*"`."
5. (1.1.8, next to `ISA-11-P4` (`ISA-11-P4.S3`)) ASSUMED. Needed sentence:
   "An entry's `step` and `phase` refer to the oracle invocation entered by
   the last oracle-call element of its `path`, or to the root invocation if
   there is none." A 0.9 alternative that also closes 1.6.7: move `step` and
   `phase` into the path elements, so that each level carries its own.
6. (1.2.2, next to `ISA-11-P4` (`ISA-11-P4.S4`)) ASSUMED. Needed: replace "two
   incomparable maximal matching entries" with "two matching entries of equal
   maximal specificity (distinct array elements, even if identical)".
7. (1.2.4.1, next to `ISA-11-P4` (`ISA-11-P4.S4`, `ISA-11-P4.S5`)) ASSUMED.
   Needed sentence: "An entry of kind `gate` is matched only against gate
   operations, `alloc` only against allocations, and `start` against every
   operation."
8. (1.2.4.2, next to `ISA-11-P4` (`ISA-11-P4.S5`)) OPEN. Needed: which
   operation identifies a cleanup for selection. For example: "A `cleanup`
   entry is matched against the cleanup's first generated operation; the
   chosen realisation applies to the whole cleanup."
9. (1.4.1, next to `ISA-11-P4` (`ISA-11-P4.S9`)) ASSUMED. Needed sentence:
   "Validation includes the maximal-match condition, and is completed before
   any operation executes."
10. (1.4.3, next to `ISA-11-P4` (`ISA-11-P4.S9`) and `ISA-9.2-P2`
    (`ISA-9.2-P2.S13`)) OPEN (refuted for unbound admission). Needed, one
    of: (a) "The maximal-match condition is checked against the step domains
    of the run's tokens; a consumer that does not know them defers the check
    to binding", or (b) record each nested token's budget in the bindings,
    e.g. `"tokens": [[B,heap,stack,depth], …]` in parameter order, so that
    the schedule alone determines Ω. Either way, also define the JCS value
    of a `token` parameter in `params_hash`.
11. (1.6.2, next to `ISA-11-P4` (`ISA-11-P4.S4`)) ASSUMED. Needed sentence:
    "The applying entry supplies every field of its decision; a field it
    omits takes its default, never the value of a less specific entry."
12. (1.6.3.2, next to `ISA-17-P8` (`ISA-17-P8.S2`)) ASSUMED. Needed: "the
    ISA version of the object" in place of "the ISA's own version".
13. (1.6.3.3, next to `ISA-11-P2` (`ISA-11-P2.S3`) and `ISA-17-P8.S2`) OPEN.
    Needed: the hash of `ref` for each ISA version, or the sentence "the
    default binding has no hash and is never equal to a written binding".
14. (1.6.4.2, next to `ISA-8.1-P10` (`ISA-8.1-P10.S1`, `ISA-8.1-P10.S2`) and
    `ISA-17-P10.S2`) OPEN. Needed sentence: "An operation is ε > 0 iff no
    `gates` entry with `epsilon` 0 matches it. N counts such operations over
    every invocation of the root region in the schedule, and the region's
    `epsilon` is `bindings.epsilon`."
15. (1.6.4.3, next to `ISA-17-P10` (`ISA-17-P10.S2`)) OPEN (refuted). Needed
    sentence: "The default share is the exact rational ε / N; it need not be
    expressible as a decimal and is never written into the schedule."
16. (1.6.5, next to `ISA-11-P3` (`ISA-11-P3.S2`)) ASSUMED. Needed sentence:
    "Decimal strings in a schedule denote the exact rational of their text;
    spelling is not significant."
17. (1.6.7, next to `ISA-11-P4` (`ISA-11-P4.S6`, `ISA-11-P4.S3`)) OPEN
    (refuted) as a property of (C), not a defect. No sentence is needed
    unless 0.9 wants every resolved decision to be writable. In that case,
    see the alternative in gap 5.
