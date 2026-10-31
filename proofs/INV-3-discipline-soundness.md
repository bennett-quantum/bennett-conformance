# INV-3: soundness of the static uncomputation discipline

Spec: ISA 0.8 (`clauses/ISA-0.8.json`). Format: `proofs/FORMAT.md`.
Depends on: INV-1 (steps cited as `INV-1:1.4` etc.).

## Informal statement

If a region body passes the static checks of §5.1 (explicit cleanup), §5.6
(borrow frames) and §6 (provenance, windows, unitary cleanup), then at run time,
under any inherited control, every `qfree` releases a bundle that is |0⟩ on every
branch and factorises from the rest of the state, every borrowed bundle comes
back with the identity applied to it and to anything entangled with it, and the
region exits with clean workspace. This is the content of T5 (`ISA-14.3-P5`, "As
§6.3") extended to borrowing. The result: proved for `perm` values under three
ASSUMED readings; **false for `other` values under the literal 0.8 text** (a
known counterexample); proved for borrow frames (both the T3 route and the
translation lemma) under one ASSUMED reading about controls.

## Precise statement

DEFINE `Seg`, `[[S]]_{Π,κ}` as in INV-1. DEFINE a region body Γ is *disciplined*
iff it is well-formed under `ISA-5.1-P1`, `ISA-5.6-P2`, `ISA-5.6-P3`, `ISA-5.6-P4`, `ISA-5.6-P5`, `ISA-5.6-P6`,
`ISA-6.1`, `ISA-6.2-P1` and `ISA-6.4-P1` (no "ill-formed" verdict).

THEOREM INV-3. For every disciplined action or permutation-action body Γ, every
inherited control context Π, classical arguments κ, entry state ψ and reference
system R (entangled with ψ's borrowed and external parts):

- (a) at every `qfree d` executed by `[[Γ]]_{Π,κ}`, the state is |0⟩_d ⊗ χ;
- (b) at every `qreturn b`, the operator applied since the matching `qborrow`
  is X ⊗ I_b ⊗ I_R for some X;
- (c) at exit the workspace, locations and local regions are clean / at image
  and the region's operation is its declared operation ⊗ identity on borrowed
  bundles and R (`ISA-5.3-P1`, `ISA-5.6-P1`).

## Hypotheses

ISA-2-P9, ISA-3.3-P3, ISA-3.5-P1, ISA-4.3-P3, ISA-4.4-P1, ISA-4.6-P1,
ISA-5.1-P1, ISA-5.2-P2, ISA-5.3-P1, ISA-5.3-P5, ISA-5.6-P1, ISA-5.6-P2,
ISA-5.6-P3, ISA-5.6-P4, ISA-5.6-P5, ISA-5.6-P6, ISA-5.6-P7, ISA-6.1-P1,
ISA-6.1-P2, ISA-6.1-P3, ISA-6.1-P9, ISA-6.1-P10, ISA-6.1-P11, ISA-6.1-P14,
ISA-6.2-P1, ISA-6.3-P1, ISA-6.4-P1, ISA-14.2-P1, ISA-14.3-P3, ISA-14.3-P5.

## Proof

```
<1>1. Every workspace bundle, location and cell version of Γ has provenance
      perm (with a dependency set S) or other.
      PROOF:
  <2>1. The §6.1 table assigns provenance to qalloc, qcopy, qload, qxor, qexch,
        qstore, qexchm, permutation/diagonal/non-diagonal gates, oracle_call,
        permutation and non-permutation action calls, qinplace and lifted rows.
        BY ISA-6.1-P1, ISA-6.1-P2, ISA-6.1-P3, ISA-6.1-P9, ISA-6.1-P10,
        ISA-6.1-P11, ISA-6.1-P14
  <2>2. `quncopy`, `qunload` and the release instructions are cleanups, not
        computations, and need no row.
        BY ISA-6.2-P1, ISA-6.4-P1, ISA-5.1-P1
  <2>3. A local `qregion` cell starts at version 0 with provenance perm and
        S = ∅ (its image gates are X gates on clean wires).
        ASSUMED: `ISA-4.6-P1` creates the cells and `ISA-6.1-P1` says "cell
        versions advance on every write", but the table has no row for
        `qregion` creation.
  <2>4. QED
        BY <2>1, <2>2, <2>3
<1>2. A perm value g computed under Π by sequence C and cleaned by the
      `reverse` realisation within its window is |0⟩ on every branch and
      factorises.
      PROOF:
  <2>1. DEFINE R = the wires of S, G = g's wires, C the exact computing
        sequence, a permutation with C|r⟩|0⟩_G = |r⟩|f(r)⟩ (controlled on Π).
        BY ISA-6.1-P1, ISA-6.1-P2, ISA-6.1-P9, <1>1
  <2>2. Every operation O between C and its cleanup is block-diagonal in the
        computational basis of R ∪ G: O = Σ_{r,x} |r,x⟩⟨r,x| ⊗ O_{r,x}.
        PROOF:
    <3>1. Diagonal gates leave values unchanged; a non-diagonal gate on a wire
          of S or g closes the window.
          BY ISA-6.1-P10, ISA-6.1-P11, ISA-6.2-P1
    <3>2. A version advance of a cell in S, or a non-permutation action call
          touching S, closes the window.
          BY ISA-6.2-P1, ISA-6.1-P14
    <3>3. Oracle calls and permutation action calls with wires of R ∪ G only as
          `ins` or `in` regions are block-diagonal on them ("ins unchanged";
          "block-diagonal in the computational basis of `ins`").
          BY ISA-5.2-P2, ISA-5.3-P1
    <3>4. A permutation change to a wire of S that is "undone before g's
          cleanup" yields, together with its undoing, a block-diagonal operator.
          ASSUMED: `ISA-6.2-P1` does not define "undone"; we read it as: the
          change and its undoing form a matched pair (INV-1 (B)) with only
          R∪G-block-diagonal operations between them.
    <3>5. A permutation gate targeting a wire of g produces a new version of g;
          the cleanup of the old version requires g's current version to be the
          one C produced.
          ASSUMED: `ISA-6.2-P1` requires "the versions that sequence read to be
          current", which taken literally includes g's pre-computation version
          (C reads its own targets) and can then never hold; we read it as: the
          versions of S that C read, and the version of g that C produced.
    <3>6. QED
          BY <3>1, <3>2, <3>3, <3>4, <3>5
  <2>3. The cleanup runs C† under the same effective control set Π.
        ASSUMED: `ISA-6.2-P1` binds a cleanup "to its exact computing
        sequence" but does not say that the effective control set is part of
        that sequence; with Π' ≠ Π, g survives on the branch Π = 1, Π' = 0.
        (Π ⊆ S fixes the control *values*, not the control *context* of the
        cleanup.) Same gap as INV-1 gap 4.
  <2>4. C† O C |r,0⟩_G|χ⟩ = |r,0⟩_G O_{r,f(r)}|χ⟩; by linearity G is |0⟩ in
        every term, so G factorises; the remaining O_{r,f(r)} are the
        "corresponding diagonal phases" of `ISA-6.3-P1`.
        BY <2>1, <2>2, <2>3, ISA-14.2-P1; algebra
  <2>5. QED
        BY <2>4, ISA-6.3-P1
<1>3. A perm value cleaned by the `recompute` realisation within its window is
      |0⟩ on every branch and factorises.
      PROOF:
  <2>1. `recompute` computes g' by the same sequence into a fresh clean bundle,
        `qxor`s it into g and uncomputes g' by the adjoint.
        BY ISA-6.4-P1
  <2>2. Within the window R holds its computation-time values on every branch,
        so g' = f(r) = g and the qxor clears g.
        BY <1>2, ISA-4.5-P3; algebra
  <2>3. The qxor is block-diagonal on R ∪ G', so the adjoint clears g' by the
        argument of 1.2.4.
        BY <2>2, <1>2
  <2>4. QED
        BY <2>2, <2>3
<1>4. An `other` value released by "the adjoint of the sequence that produced
      it" is |0⟩ on every branch.
      PROOF:
  <2>1. If no operation between the producing sequence A and adj(A) writes a
        wire A writes or reads one, adj(A) restores it.
        BY INV-1:1.4, ISA-6.2-P1
  <2>2. ISA 0.8 does not require that hypothesis, and without it the claim is
        false: `h d0; ccx d0 p d1; cx d1 t; ccx d0 p d1; h d0; qfree d0`
        (t a `mut` interface wire) passes §6 literally (d0 other by
        `ISA-6.1-P11`; d1 perm with d0 ∈ S by `ISA-6.1-P9`, cleaned within its
        window; interface wires carry no obligation; d0 released by the adjoint
        `h d0`). For p = 1, t = 0 the state before release is
        (|0⟩(|0⟩+|1⟩) + |1⟩(|0⟩−|1⟩))/2 on (d0, t): d0 is not clean.
        OPEN (refuted): soundness for `other` values; SPEC-QUESTIONS 25.
  <2>3. Under the reading of SPEC-QUESTIONS 25 ("only while no other live value
        or outstanding diagonal phase depends on it") the hypothesis of <2>1
        holds.
        OPEN: needs a definition of "depends" that covers interface `mut`
        wires and diagonal phases; not proved here.
  <2>4. QED
        BY <2>1 (only in the case <2>1 covers)
<1>5. Every completed borrow lifetime acts as X ⊗ I_b ⊗ I_R.
      PROOF:
  <2>1. Borrowed wires are touched only inside borrow frames, each
        permutation-only; outside frames the lifetime does not touch b.
        BY ISA-5.6-P2
  <2>2. A frame established by a T3 claim over its full domain (every value
        of b included) satisfies P_F = P' ⊗ I_B on span(D), hence on every
        state of b, entangled with R or not.
        BY ISA-5.6-P2, ISA-14.3-P3; algebra
  <2>3. A frame of the form V; W; V⁻¹; W satisfying (i)–(iv) satisfies
        P_F = P' ⊗ I_B.
        PROOF:
    <3>1. V's gates are X-family flips with targets in B and controls in X,
          X ∩ B = ∅; they commute and are involutions; V maps b ↦ b ⊕ v(X) and
          leaves X fixed; V⁻¹ (exact reverse) undoes it.
          BY ISA-5.6-P3, ISA-5.6-P6, ISA-4.4-P1; algebra
    <3>2. No target of W is a control of W, in X or in B, so W's gates
          commute, each is an involution, and every control value is constant
          during W.
          BY ISA-5.6-P4; algebra
    <3>3. Each W gate g does t_g ⊕= c_g(Y, X)·b_{j(g)} (one B control) or
          t_g ⊕= c_g(Y, X) (none).
          BY ISA-5.6-P5, <3>2; algebra
    <3>4. Over V; W; V⁻¹; W the net map is t_g ⊕= c_g(Y, X)·v_{j(g)}(X) (the
          terms without a B control cancel), b and X are restored, and nothing
          depends on b: the dirty-ancilla identity of `ISA-5.6-P7`.
          BY <3>1, <3>2, <3>3, ISA-5.6-P7; algebra
    <3>5. The active predicate and inherited control context count as controls
          of every gate of V and W in (i)–(iii), and V⁻¹ runs under the same
          effective control set as V.
          ASSUMED: `ISA-3.5-P1` controls every gate on the active predicate,
          but `ISA-5.6-P3`, `ISA-5.6-P4`, `ISA-5.6-P5`, `ISA-5.6-P6` speak of a gate's controls without
          saying whether the predicate is included; if V⁻¹ ran under a
          different set, "exact reverse sequence" would hold lexically but not
          as operators.
    <3>6. QED
          BY <3>4, <3>5
  <2>4. Maximality of a frame inside a `qloop`/`qif` body is judged within the
        body's static instruction sequence.
        ASSUMED: "maximal contiguous" in `ISA-5.6-P2` does not say whether it
        is judged on the unrolled execution or on the static sequence
        (SPEC-QUESTIONS 30); either reading gives a factorising frame, but the
        frames differ, so certificate keys must agree on one.
  <2>5. A product of operators of the form X_k ⊗ I_b, and of operators not
        touching b, has the form X ⊗ I_b.
        BY <2>1; algebra
  <2>6. QED
        BY <2>2, <2>3, <2>4, <2>5
<1>6. Under an inherited control w the conclusions hold on both branches.
      PROOF:
  <2>1. In `ctl w @a`, workspace, regions and flags are allocated
        unconditionally and every gate of the body and its cleanup runs under
        w; on the branch w = 0 no gate acts, so workspace stays |0⟩ / at image.
        BY ISA-5.3-P5, ISA-3.5-P1
  <2>2. On the branch w = 1 the body runs with Π ∪ {w}; apply <1>2–<1>5.
        BY <1>2, <1>3, <1>4, <1>5, ISA-2-P9
  <2>3. Cleanup schedules are keyed by inherited control count and every
        admitted realisation is unitary.
        BY ISA-6.4-P1
  <2>4. QED
        BY <2>1, <2>2, <2>3; algebra (block-diagonal in w)
<1>7. Every `qfree d` in a disciplined body releases a bundle whose current
      version is its `qalloc` version or the output of a §6.4 cleanup.
      ASSUMED: `ISA-4.3-P3` states the dynamic precondition ("clean on every
      basis configuration") and `ISA-5.1-P1` requires matching by "explicit
      cleanup instructions", but no clause states how the static discipline
      decides that a particular `qfree` meets the precondition.
<1>8. QED
      BY <1>1, <1>2, <1>3, <1>5, <1>6, <1>7 for (a)–(c) on perm values and
      borrowed bundles; (a) and (c) for `other` values only in the case of
      1.4.1 (the general case is OPEN, refuted for the literal text).
```

## Witnesses

| Witness | Steps |
|---|---|
| `suite/cases/r9-codex-oracle`, `r9-sol-oracle`, `r9-sol-qinplace`, `r9-loop` (borrow/CX/return after a cleaned control; tagged ISA-5.6, ISA-6.4) | 1.5, 1.2 |
| `suite/cases/actions` (`flip`: perm copy cleaned by reverse) | 1.2 |
| `tests/discipline.rs`: `cleanup_adjoint_cannot_skip_an_entangling_reader` (the 1.4.2 counterexample, rejected under SPEC-QUESTIONS 25), `other_scratch_requires_the_exact_adjoint_but_accepts_h_h`, `cleanup_pins_predicate_versions_and_distinguishes_qif_polarity`, `local_regions_require_explicit_restoration_and_load_pins_cell_versions` | 1.4, 1.2.3, 1.1.3 |
| `tests/r1_discipline.rs`: `fable_r1_b3_predicate_flip_closes_the_window`, `codex_b03_predicate_versions_and_windows_start_at_the_computation`, `fuzz_witnesses_of_entangling_reads_are_rejected`, `r2_complementary_diagonal_phases_do_not_cancel`, `r2_exchange_inverse_must_cancel_both_outputs_together` | 1.2.2, 1.4.2 |
| `tests/r1_fuzz_discipline.rs`: `accepted_random_actions_run_exactly_clean`, `accepted_random_actions_run_exactly_clean_sweep` (every accepted program runs clean: the empirical form of (a)–(c) for the interpreter's stricter discipline) | 1.8 |
| `tests/borrow.rs`: `translation_lemma_checks_all_four_required_negative_patterns`, `translation_pattern_returns_an_entangled_borrowed_system`, `borrow_frame_exhaustion_rejects_hidden_dependence_on_borrowed_bits`, `exhaustive_borrow_identity_need_not_be_adjacent_gate_cancellation` | 1.5.2, 1.5.3 |
| `tests/r1_borrow.rs`: `borrow_frames_inside_bounded_loops_are_composed_not_unrolled`, `r4_loop_local_returns_cannot_be_restored_by_later_borrowings` | 1.5.4, 1.5.5 |
| `tests/r1_certificates.rs::muse_r5_translation_frame_maximality_is_within_its_static_sequence` | 1.5.4 |

## Council blockers as instances

Rounds 9–10 blockers (SOL-R9-B01, R9-B01, R9-B02, R10-B02) were *completeness*
failures of the reference checker: it rejected programs that are clean, because
it did not recognise matched pairs (INV-1 1.2.5, 1.2.6, 1.4) or keyed controls
by ordered stacks (INV-1 1.1.2). They do not refute INV-3; they show that the
discipline's "exact computing sequence" (1.2.3) needs the matching key of INV-1
gap 4 to be decidable the same way by every implementation. The SPEC-QUESTIONS 25
fuzz finding is the one known *soundness* failure, step 1.4.2.

## Spec gaps

1. (1.1.3, next to `ISA-6.1-P1`, `ISA-4.6-P1`) ASSUMED. Needed row: "| `qregion`
   creation | every cell: version 0, perm, S = ∅ |".
2. (1.2.2.4, next to `ISA-6.2-P1`) ASSUMED. Needed sentence: "A permutation change
   to a wire of S is undone when it and its undoing form a matched pair (§5.3)
   and every operation between them leaves the computational-basis values of S
   and g unchanged."
3. (1.2.2.5, next to `ISA-6.2-P1`) ASSUMED. Needed sentence: "A cleanup requires
   the versions of S that the computing sequence read, and the version of g that
   it produced, to be current."
4. (1.2.3, next to `ISA-6.2-P1`) ASSUMED. Needed sentence: "A cleanup runs under
   the effective control set (§2) of its computing sequence."
5. (1.4.2, next to `ISA-6.2-P1`) OPEN (refuted). Needed sentence (SPEC-QUESTIONS 25):
   "An `other` value is released only by the adjoint of its computing sequence,
   and only while no other live value, interface wire or outstanding diagonal
   phase depends on it."
6. (1.4.3, next to `ISA-6.1-P1`) OPEN. Needed: a definition of "depends on" that
   extends dependency sets to interface `mut` wires and to diagonal phases.
7. (1.5.3.5, next to `ISA-5.6-P3`) ASSUMED. Needed sentence: "In (i)–(iii) a
   gate's controls include the effective control context; V⁻¹ runs under the
   same effective control set as V."
8. (1.5.4, next to `ISA-5.6-P2`) ASSUMED. Needed sentence (SPEC-QUESTIONS 30):
   "Maximality of a frame inside a `qloop` or `qif` body is judged within that
   body's instruction sequence."
9. (1.7, next to `ISA-4.3-P3`, `ISA-5.1-P1`) ASSUMED. Needed sentence: "`qfree d`
   is well-formed iff the current version of every wire of d is its `qalloc`
   version or the output of a §6.4 cleanup of the value that occupied it."
