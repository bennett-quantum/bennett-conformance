# INV-1: matched compute/uncompute pairs are the identity

Spec: ISA 0.8 (`clauses/ISA-0.8.json`). Format: `proofs/FORMAT.md`.

## Informal statement

Running a well-formed piece of quantum code and then its adjoint, as the adjoint
table builds it, under the same effective control, returns every wire that was
live before to its state, releases every wire the piece allocated clean, and does
nothing to borrowed bundles or to any reference system entangled with them. This
holds with opaque calls inside (an oracle is its own adjoint; `qinplace @g @ginv`
is undone by `qinplace @ginv @g`), and also when the two halves of a matched pair
are separated by code that does not touch what the pair writes. The effective
control is a set: two calls whose predicate stacks list the same wires in a
different order, or with repeats, are under the same control.

## Precise statement

DEFINE `Seg` = finite sequences of the instructions admitted in an action body
(`ISA-5.3-P2`), with `ctl w <call>` and `adj action_call` (`ISA-4.7-P1`).
DEFINE `adj(S)` = the adjoint of S given by the Adjoint table `ISA-5.3-P4`
("reverse order, instruction replaced, versions renumbered").
DEFINE for a set Π of wires (an effective control context, `ISA-2-P9`) and a fixed
assignment κ of classical arguments, tokens included, fixed at region entry
(`ISA-3.1-P2`): `[[S]]_{Π,κ}` = the operator S realises on the region state of
`ISA-14.2-P1`, with every gate at its specified unitary (`ISA-4.4-P1`, ε = 0).
States are compared on named linear values (`ISA-3.3-P2`): a re-allocated bundle
is identified with the name it is bound to, not with a physical wire.
DEFINE S is *closed* iff every `qreturn` in S has its matching `qborrow` in S.

THEOREM INV-1. For every S ∈ Seg such that S; adj(S) is well-formed (`ISA-3.3-P2`,
`ISA-3.3-P3`, `ISA-5.1-P1`) and S is closed, every Π, κ, every state ψ of the
wires live at entry of S, and every reference system R:

- (A) `[[S; adj(S)]]_{Π,κ} (ψ ⊗ φ_R) = ψ ⊗ φ_R`, every bundle allocated in S is
  released clean on every basis configuration, and borrowed bundles hold their
  entry values.
- (B) If A is one `oracle_call`, `qinplace` or `action_call` and A' is adj(A) with
  the same callee(s), the same κ, the same operand wires at the versions A
  produced or read, and an equal effective control *set*, and M ∈ Seg writes no
  wire A reads or writes and reads no wire A writes, then
  `[[A; M; A']] = [[M]]`.
- (C) A `lift.oracle_call` followed by its `lift.oracle_uncall` per `ISA-5.5-P1`
  returns `tgt` and `fr'` to their values before the call.

## Hypotheses

ISA-2-P9, ISA-3.1-P2, ISA-3.2-P8, ISA-3.3-P2, ISA-3.3-P3, ISA-3.5-P1, ISA-4.3-P1,
ISA-4.3-P2, ISA-4.3-P3, ISA-4.3-P4, ISA-4.4-P1, ISA-4.4-P2, ISA-4.5-P1, ISA-4.5-P2, ISA-4.5-P3, ISA-4.5-P4, ISA-4.5-P5,
ISA-4.6-P1, ISA-4.6-P2, ISA-4.6-P3,
ISA-4.6-P4, ISA-4.6-P5, ISA-4.6-P6, ISA-4.6-P7, ISA-4.6-P8, ISA-4.6-P9, ISA-4.7-P1, ISA-5.1-P1, ISA-5.2-P2, ISA-5.2-P4, ISA-5.3-P1,
ISA-5.3-P2, ISA-5.3-P4, ISA-5.3-P5, ISA-5.4-P1, ISA-5.5-P1, ISA-5.6-P1,
ISA-5.6-P2, ISA-14.2-P1.

## Proof

```
<1>1. [[S]]_{Π,κ} depends on Π only as a set, and control composes:
      C_Π(U) C_Π(V) = C_Π(UV), C_Π(U)† = C_Π(U†).
      PROOF:
  <2>1. The active predicate is the conjunction of the stack wires, and every
        unitary instruction executes controlled on it; the inherited context is
        added the same way.
        BY ISA-3.2-P8, ISA-3.5-P1, ISA-2-P9, ISA-4.7-P1, ISA-5.3-P5
  <2>2. A conjunction depends only on the set of its conjuncts (order and
        repetition irrelevant); `ISA-2-P9`: "duplicates count once".
        BY <2>1, ISA-2-P9; algebra
  <2>3. No instruction modifies a wire of the effective control context it
        runs under: "every wire of the effective control context is distinct
        from every target and from every consumed or produced operand".
        BY ISA-3.3-P3
  <2>4. Allocation, borrowing, release and region creation are unconditional;
        only gates are controlled.
        BY ISA-3.5-P1, ISA-5.3-P5
  <2>5. QED
        BY <2>2, <2>3, <2>4; algebra (a projector onto Π=1 that commutes with
        both operators because neither writes Π)
<1>2. For every single non-lifetime instruction i, [[i; adj(i)]]_{Π,κ} = id.
      PROOF:
  <2>1. Gates: rx/ry/rz(θ) then (−θ), s/sdg, t/tdg, and the self-adjoint
        x cx ccx mcx swap h cz compose to I exactly (ε = 0).
        BY ISA-4.4-P1, ISA-5.3-P4; algebra
  <2>2. qxor, qexch: products of cx resp. swap on pairwise distinct wires;
        each factor is an involution and the factors commute.
        BY ISA-4.5-P3, ISA-4.5-P4, ISA-3.3-P3; algebra
  <2>3. qstore, qexchm: for each basis value of the address a exactly one
        branch `a == i` is active and a is not written, so on that branch the
        instruction is one qxor resp. qexch, an involution.
        BY ISA-4.6-P4, ISA-4.6-P5, ISA-3.3-P3, <2>2; algebra
  <2>4. qcopy;quncopy and qload;qunload: the cx fan-out applied twice is I, so
        d is clean on every basis configuration and the release is legal.
        BY ISA-4.5-P1, ISA-4.5-P2, ISA-4.6-P2, ISA-4.6-P3, ISA-4.3-P3, ISA-14.2-P1
  <2>5. oracle_call ; oracle_call (same callee, κ, operands): on every basis
        x ∈ D the oracle maps (ins, tgt, fr) to (ins, tgt ⊕ payload, fr ⊕ code)
        and leaves workspace, locations and `in` regions clean or unchanged;
        hence it is an involution, as `ISA-5.2-P2` states ("an involution on
        (ins, tgt, fr) and its own adjoint"), and acts as identity on
        borrowed bundles and entangled external systems.
        BY ISA-5.2-P2, ISA-5.2-P4, ISA-4.7-P1, ISA-5.3-P4, ISA-5.6-P1; algebra
  <2>6. qinplace @g @ginv tk x ; qinplace @ginv @g tk x = id, provided the
        `ISA-4.5-P5` certificates hold under tk.
        PROOF:
    <3>1. Under tk with fault code FAULT_NONE (= 0) on every input, the
          expansion "t = qalloc; f = qalloc; oracle_call @g (x) t f; qexch x t;
          oracle_call @ginv (x) t f; qfree t; qfree f" maps x ↦ g(x) with t and
          f clean at release, because ginv(g(x)) = x.
          BY ISA-4.5-P5, ISA-5.2-P2, ISA-5.4-P1; algebra
    <3>2. The adjoint instruction realises x ↦ ginv(x) by <3>1 with g, ginv
          exchanged; ginv ∘ g = id.
          BY <3>1, ISA-5.3-P4, ISA-4.5-P5
    <3>3. When tk does not make both oracles fault-free (e.g. B = 0, where the
          code is FAULT_BUDGET_STEPS), the instruction has no defined meaning.
          ASSUMED: `ISA-4.5-P5` requires certificates "under tk" but says
          neither when they are checked nor what happens if they fail; we read
          the instruction as refused before any state change (SPEC-QUESTIONS 26).
    <3>4. QED
          BY <3>1, <3>2, <3>3
  <2>7. qpred w … qunpred w: the pair is reversed as a whole and does not
        change the state; it only changes Π for the enclosed code.
        BY ISA-5.3-P4, ISA-3.5-P1
  <2>8. QED
        BY <2>1, <2>2, <2>3, <2>4, <2>5, <2>6, <2>7, <1>1
<1>3. ASSUME S = i; T with T satisfying (A) (induction hypothesis on the
      well-founded measure (call depth, nesting depth, length), well-founded
      because "Action call graphs are acyclic" (ISA-5.1-P1))
      PROVE  S satisfies (A)
      PROOF:
  <2>1. adj(i; T) = adj(T); adj(i).
        BY ISA-5.3-P4
  <2>2. Case i is a non-lifetime, non-compound instruction.
        BY <2>1, <1>2, induction hypothesis; algebra
  <2>3. Case i = `d = qalloc` (or `qregion` creation, function entry only): after
        T; adj(T) = id, d is |0⟩ (resp. at its image) on every basis
        configuration, so adj(i) = qfree (resp. release with image gates
        reversed) removes a clean factor.
        BY <2>1, ISA-4.3-P1, ISA-4.3-P3, ISA-4.6-P1, ISA-14.2-P1
  <2>4. Case i = `qfree d`: the composite ends `qfree d; … ; d' = qalloc`;
        d was clean, d' is clean, and d' is bound to d's name.
        BY <2>1, ISA-4.3-P1, ISA-4.3-P3, ISA-14.2-P1, ISA-3.3-P2
  <2>5. Case i = `b = qborrow`; by closedness T = T1; qreturn b; T2.
        PROOF:
    <3>1. Between qborrow and qreturn, b is touched only inside borrow frames,
          each acting as P' ⊗ I_B; hence T1 acts as X ⊗ I_b.
          BY ISA-5.6-P2, ISA-5.6-P1
    <3>2. The composite is qborrow b; T1; qreturn b; [T2; adj(T2)]; qborrow b';
          adj(T1); qreturn b'. The bracket is id by hypothesis; b' may be other
          physical wires in another unknown state.
          BY <2>1, induction hypothesis, ISA-4.3-P2, ISA-4.3-P4
    <3>3. (X⁻¹ ⊗ I_b') (X ⊗ I_b) = id on everything but the borrowed pool,
          which is returned untouched.
          BY <3>1, <3>2, ISA-14.2-P1; algebra
    <3>4. QED
          BY <3>3
  <2>6. Case i = `qif w {A}{B}`.
        PROOF:
    <3>1. qif is its expansion qpred w; A; qunpred w; x w; qpred w; B;
          qunpred w; x w, i.e. C_{¬w}(B) C_w(A).
          BY ISA-4.6-P7, <1>1
    <3>2. The flat adjoint of that expansion satisfies (A).
          BY <3>1, <1>2, induction hypothesis (smaller nesting depth)
    <3>3. The table's row `qif w {A}{B} ↦ qif w {adj A}{adj B}` equals the flat
          adjoint of <3>2 when each arm's lifetime instructions are
          self-contained (then C_w(·) and C_{¬w}(·) act on one fixed space and
          commute).
          BY ISA-5.3-P4, ISA-3.5-P1; algebra
    <3>4. Otherwise the row is not even well-formed: for
          `qif p { d = qalloc any 1 } { qfree d }` it yields
          `qif p { qfree d } { d = qalloc any 1 }`, which consumes d before it
          is produced (ISA-3.3-P2).
          OPEN (refuted): the qif row of `ISA-5.3-P4` is wrong for cross-arm
          lifetimes; SPEC-QUESTIONS 28.
    <3>5. QED
          BY <3>2, <3>3 (the table row is valid only under <3>3's condition)
  <2>7. Case i = `qloop n {A}`: qloop is A^n and its adjoint (adj A)^n, so the
        composite is id by n uses of the hypothesis.
        PROOF:
    <3>1. [[qloop n {A}]] = [[A]]^n and adj = qloop n {adj A}.
          BY ISA-4.6-P8, ISA-5.3-P4
    <3>2. Each visit resolves the body's names against the bindings current at
          that visit's entry, and its successor bindings feed the next visit.
          ASSUMED: `ISA-3.3-P2` says "Regions are threaded the same way" but
          nothing says how a loop body's linear names are threaded from one
          visit to the next (council R9-B02).
    <3>3. QED
          BY <3>1, <3>2, induction hypothesis; algebra
  <2>8. Case i = `qloop n while cond {A}`.
        PROOF:
    <3>1. Flags c_1 = cond[0], c_{i+1} = c_i ∧ cond[i] are functions of cond
          only, and neither the body nor anything between the forward loop
          and its adjoint writes cond.
          BY ISA-4.6-P9, ISA-3.3-P3
    <3>2. On each basis value of cond with k leading ones, the forward loop is
          A^k and the table's adjoint ("flags computed forward …; adj A for
          i = n..1 under c_i; flags uncomputed") is (adj A)^k; flags are clean
          after each.
          BY <3>1, ISA-5.3-P4, <1>1; algebra
    <3>3. QED
          BY <3>2, induction hypothesis
  <2>9. Case i = `action_call @b` (or `ctl w`, `adj action_call`): its adjoint
        `adj action_call @b` executes adj(body of @b); the body has smaller
        call depth.
        BY ISA-4.7-P1, ISA-5.3-P1, ISA-5.3-P4, ISA-5.3-P5, ISA-5.1-P1,
        induction hypothesis, <1>1
  <2>10. QED
        BY <2>2, <2>3, <2>4, <2>5, <2>6, <2>7, <2>8, <2>9 (cases are exhaustive
        over ISA-5.3-P2; `qreturn` first is excluded by closedness)
<1>4. Part (B): [[A; M; A']] = [[M]].
      PROOF:
  <2>1. An operator that does not write a wire r is block-diagonal in r's
        computational basis: A = Σ_r |r⟩⟨r| ⊗ A_r on its read-only wires.
        BY ISA-14.2-P1, ISA-4.4-P1; algebra
  <2>2. Two operators whose write sets are disjoint and neither of which reads
        the other's writes commute.
        BY <2>1; algebra
  <2>3. A' = adj(A) as an operator only if the callee(s), κ, operand versions
        and effective control *set* agree.
        BY <1>1, <1>2, ISA-5.3-P4
  <2>4. The spec defines such a matching key only for
        lift.oracle_call/lift.oracle_uncall (`ISA-5.5-P1`: "the same token,
        callee, `ins` and `mem` values … the pair shares a dynamic invocation
        identity"); for structured oracle_call, qinplace and action_call pairs
        the key in (B) is our definition.
        ASSUMED: matching key for structured pairs (council SOL-R9-B01, R9-B01,
        R10-B02).
  <2>5. QED
        BY <2>2, <2>3, <2>4, <1>2; algebra (A M A' = M A A' = M)
<1>5. Part (C): lifted pairs restore tgt and fr'.
      PROOF:
  <2>1. The call XORs payload(F_B(x)) into tgt and code(F_B(x)) into fr'.
        BY ISA-5.2-P2, ISA-5.5-P1
  <2>2. The uncall uses the same token, callee, ins and mem values (no
        intervening row writes them, suite F), so F_B(x) is the same and the
        same values are XORed again.
        BY ISA-5.5-P1, ISA-5.2-P2; algebra
  <2>3. QED
        BY <2>1, <2>2
<1>6. QED
      BY <1>3 (with the empty S as base, trivially id), <1>4, <1>5
```

## Witnesses

| Witness | Steps |
|---|---|
| `suite/cases/r9-codex-oracle`, `suite/cases/r9-sol-oracle` (H; O; O; H then borrow/CX/return is identity) | 1.2.5, 1.3 |
| `suite/cases/r9-sol-qinplace` (B ≥ 1 identity; B = 0 refused) | 1.2.6, 1.2.6.3 |
| `suite/cases/r9-loop` (mixed loop re-allocating c) | 1.3.7, 1.3.7.2 (ASSUMED) |
| `suite/cases/actions` (load/unload, store pairs, qexchm pairs, adjoint after action is identity) | 1.2.3, 1.2.4, 1.3 |
| `suite/cases/nested` (`cancel`: call then uncall returns tgt, fr to zero) | 1.5 |
| `tests/r1_borrow.rs`: `r9_opaque_adjoint_restores_borrow_control`, `r9_matched_opaque_oracle_restores_outer_fact`, `r9_qinplace_adjoint_restores_borrow_control`, `r9_qinplace_zero_budget_rejects_before_mutation`, `r9_mixed_loop_threads_reallocated_bindings`, `r10_equivalent_control_sets_preserve_opaque_inverse` | 1.1, 1.2.5, 1.2.6, 1.3.7 |
| `tests/r1_qinplace.rs`: `b07_zero_zero_pair_is_rejected_before_mutation`, `b07_not_not_pair_executes_controlled_and_adjoint`, `r2_qinplace_explicit_adjoint_cleans_workspace` | 1.2.6 |
| `tests/r1_discipline.rs::r2_qif_cross_arm_unconditional_lifetimes_reverse_the_expansion` | 1.3.6 (OPEN 1.3.6.4) |
| `tests/quantum_control.rs::coherent_while_is_prefix_and_padding_with_an_exact_adjoint` | 1.3.8 |
| `tests/quantum.rs`: `every_gate_and_controlled_global_phase`, `bell_state_born_branches_and_adjoint` | 1.2.1, 1.1 |
| `tests/r1_fuzz_discipline.rs`: `generated_r9_opaque_histories_match_independent_identity`, `generated_r9_loop_bindings_match_independent_identity`, `generated_r10_effective_control_sets_match_independent_identity` | 1.1, 1.2.5, 1.3 |

## Council blockers as instances

The theorem is a property of the spec; the round 9–10 blockers were an
implementation (the reference checker) failing to *use* a step, so it rejected
programs that INV-1 proves are the identity.

- SOL-R9-B01, R9-B01 (matched opaque oracle pairs erase the outer H record):
  the checker did not apply 1.2.5 (oracle involution) inside 1.3. The qinplace
  half of SOL-R9-B01 is 1.2.6.
- R9-B02 (mixed loop reuses first-visit bindings): the checker violated the
  reading of 1.3.7.2.
- R10-B02 (lexical predicate stack vs effective-control set): the checker keyed
  inverses by ordered stacks, against 1.1.2.
- SOL-R11-B03 (fixture comment claimed B = 0 qinplace identity): the comment
  ignored the precondition of 1.2.6; the runtime refusal matches 1.2.6.3.

## Spec gaps

1. (1.2.6.3, next to `ISA-4.5-P5`) ASSUMED. Needed sentence: "An executor
   checks the conditions on `@g` and `@ginv` under the call's token before any
   state change; if they do not hold the call is refused (CC/R trap) and has no
   effect."
2. (1.3.6.4, next to `ISA-5.3-P4`) OPEN (refuted). Needed sentence: "The
   row `qif w {A}{B} ↦ qif w {adj A}{adj B}` applies when each arm's allocations
   and releases are matched within that arm; otherwise the adjoint is the
   adjoint of the complete §4.6 expansion, unconditional lifetime instructions
   included, in reverse order."
3. (1.3.7.2, next to `ISA-3.3-P2` and `ISA-4.6-P8`) ASSUMED. Needed
   sentence: "A `qloop` body is threaded like a block: each visit resolves names
   against the bindings live at its entry, and the bindings live at its exit are
   the entry bindings of the next visit; they MUST agree in names and types."
4. (1.4.4, next to `ISA-5.3-P4` and `ISA-6.2-P1`) ASSUMED. Needed sentence:
   "Two operations form a matched pair iff the second is the §5.3 adjoint of the
   first with the same callee(s), the same classical arguments (token
   included), the same operand wires at the versions the first read or
   produced, and equal effective control sets (§2); predicate-stack order and
   repetition are irrelevant."
